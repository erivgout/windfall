//! Rice coding of what is left of a block once its predictor has been taken
//! off.
//!
//! The residual is cut into `2^order` partitions of equal length, each with
//! a parameter of its own. A value is folded to an unsigned number, whose
//! bits above the parameter are written in unary and the rest as they are.

use super::bits::BitWriter;

/// How the residual of one subframe is coded.
#[derive(Debug, Default)]
pub(super) struct RicePlan {
    /// The residual is in `2^order` partitions.
    pub order: u32,
    /// The parameter of each partition.
    pub params: Vec<u8>,
    /// The most bits the coded residual takes, the partitions' parameters
    /// included.
    pub bits: u64,
}

/// What the format lets a plan use.
#[derive(Debug, Clone, Copy)]
pub(super) struct RiceLimits {
    /// The highest partition order to try.
    pub max_order: u32,
    /// Bits a parameter is stored in: four, or five for audio of more than
    /// 16 bits, whose residual can be too large for a four-bit parameter.
    pub param_bits: u32,
}

impl RiceLimits {
    /// The largest parameter. The one above it marks a partition stored
    /// unencoded, which this encoder never writes.
    fn max_param(self) -> u32 {
        (1 << self.param_bits) - 2
    }
}

/// Memory [`plan`] works in, kept from block to block.
#[derive(Debug, Default)]
pub(super) struct RiceScratch {
    sums: Vec<u64>,
    params: Vec<u8>,
}

/// Folds a signed value into an unsigned one: 0, -1, 1, -2 become 0, 1, 2, 3.
fn fold(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)) as u32
}

/// Finds the partition order and parameters that code `residual` in the
/// fewest bits. The residual belongs to a block of `residual.len() +
/// predictor_order` samples, whose first `predictor_order` are stored
/// apart, so the first partition is that much shorter than the others.
///
/// The cost of a partition is counted from the sum of its values, which
/// can only overstate it, so `bits` is an upper bound.
pub(super) fn plan(
    residual: &[i32],
    predictor_order: usize,
    limits: RiceLimits,
    scratch: &mut RiceScratch,
    out: &mut RicePlan,
) {
    let block = residual.len() + predictor_order;
    let mut order = limits.max_order.min(block.trailing_zeros());
    while order > 0 && (block >> order) <= predictor_order {
        order -= 1;
    }

    let sums = &mut scratch.sums;
    sums.clear();
    let length = block >> order;
    let mut rest = residual;
    for partition in 0..1_usize << order {
        let count = if partition == 0 {
            length - predictor_order
        } else {
            length
        };
        let (values, later) = rest.split_at(count);
        sums.push(values.iter().map(|&value| u64::from(fold(value))).sum());
        rest = later;
    }

    out.bits = u64::MAX;
    loop {
        let length = (block >> order) as u64;
        let partitions = 1_usize << order;
        scratch.params.clear();
        let mut bits = 0;
        for (partition, &sum) in sums[..partitions].iter().enumerate() {
            let count = if partition == 0 {
                length - predictor_order as u64
            } else {
                length
            };
            let (param, cost) = best_param(sum, count, limits.max_param());
            scratch.params.push(param);
            bits += cost + u64::from(limits.param_bits);
        }
        if bits < out.bits {
            out.bits = bits;
            out.order = order;
            out.params.clear();
            out.params.extend_from_slice(&scratch.params);
        }
        if order == 0 {
            break;
        }
        // Two partitions of this order make one of the next.
        for partition in 0..partitions / 2 {
            sums[partition] = sums[2 * partition] + sums[2 * partition + 1];
        }
        order -= 1;
    }
}

/// The parameter that codes `count` values adding up to `sum` in the fewest
/// bits, and that many bits at most.
fn best_param(sum: u64, count: u64, max_param: u32) -> (u8, u64) {
    let mut best = (0, u64::MAX);
    for param in 0..=max_param {
        let cost = count * (u64::from(param) + 1) + (sum >> param);
        if cost < best.1 {
            best = (param as u8, cost);
        }
    }
    best
}

/// Writes the coded residual of a subframe.
pub(super) fn write(
    out: &mut BitWriter,
    residual: &[i32],
    predictor_order: usize,
    plan: &RicePlan,
    limits: RiceLimits,
) {
    out.put(u32::from(limits.param_bits == 5), 2);
    out.put(plan.order, 4);
    let length = (residual.len() + predictor_order) >> plan.order;
    let mut rest = residual;
    for (partition, &param) in plan.params.iter().enumerate() {
        let count = if partition == 0 {
            length - predictor_order
        } else {
            length
        };
        let (values, later) = rest.split_at(count);
        rest = later;
        let param = u32::from(param);
        out.put(param, limits.param_bits);
        for &value in values {
            let folded = fold(value);
            out.put_unary(folded >> param);
            out.put(folded, param);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: RiceLimits = RiceLimits {
        max_order: 8,
        param_bits: 4,
    };

    fn planned(residual: &[i32], predictor_order: usize, limits: RiceLimits) -> RicePlan {
        let mut plan = RicePlan::default();
        super::plan(
            residual,
            predictor_order,
            limits,
            &mut RiceScratch::default(),
            &mut plan,
        );
        plan
    }

    /// Bits `write` puts out, without the six that name the method and the
    /// order.
    fn written_bits(residual: &[i32], predictor_order: usize, plan: &RicePlan) -> u64 {
        let mut out = BitWriter::default();
        write(&mut out, residual, predictor_order, plan, LIMITS);
        // Enough ones to see where the last byte's padding begins.
        out.put(1, 1);
        out.align();
        let bytes = out.bytes();
        let last = bytes[bytes.len() - 1];
        (bytes.len() as u64 * 8) - u64::from(last.trailing_zeros()) - 1 - 6
    }

    #[test]
    fn folding_alternates_signs() {
        let folded: Vec<u32> = [0, -1, 1, -2, 2, i32::MAX, i32::MIN + 1]
            .into_iter()
            .map(fold)
            .collect();
        assert_eq!(folded, [0, 1, 2, 3, 4, u32::MAX - 1, u32::MAX - 2]);
    }

    #[test]
    fn the_stated_bits_are_never_fewer_than_the_written_ones() {
        let mut state = 0x1234_5678_u32;
        let mut noise = |spread: u32| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((state >> 8) % (2 * spread + 1)) as i32 - spread as i32
        };
        for (block, predictor_order) in [(4096, 0), (4096, 8), (4095, 4), (48, 12), (17, 2), (1, 0)]
        {
            for spread in [0, 1, 5, 300, 20_000] {
                // Loud at the start and quiet after, which partitions help.
                let residual: Vec<i32> = (0..block - predictor_order)
                    .map(|index| {
                        noise(if index < block / 4 {
                            spread
                        } else {
                            spread / 16
                        })
                    })
                    .collect();
                let plan = planned(&residual, predictor_order, LIMITS);
                assert_eq!(plan.params.len(), 1 << plan.order);
                let written = written_bits(&residual, predictor_order, &plan);
                assert!(written <= plan.bits, "{written} > {}", plan.bits);
                assert!(
                    written + residual.len() as u64 >= plan.bits,
                    "{written} is far below {}",
                    plan.bits
                );
            }
        }
    }

    #[test]
    fn a_block_that_changes_level_is_cut_into_partitions() {
        let residual: Vec<i32> = (0..4096)
            .map(|index| if index < 1024 { 1_000 } else { 1 })
            .collect();
        let split = planned(&residual, 0, LIMITS);
        assert!(split.order >= 2);
        assert!(split.params[0] > split.params[split.params.len() - 1]);
        let whole = planned(
            &residual,
            0,
            RiceLimits {
                max_order: 0,
                ..LIMITS
            },
        );
        assert_eq!(whole.order, 0);
        assert!(split.bits < whole.bits);
    }

    #[test]
    fn an_odd_block_gets_one_partition() {
        // 4095 cannot be halved, and 16 samples cut in four would leave
        // nothing in the first partition after an order-4 predictor.
        assert_eq!(planned(&[3; 4095], 0, LIMITS).order, 0);
        assert!(planned(&[3; 12], 4, LIMITS).order <= 1);
    }
}

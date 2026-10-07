//! The predictors of a FLAC subframe: the five fixed polynomials, and
//! linear prediction fitted to the block.
//!
//! Everything that decides a bit of the file is done in integers or in
//! plain floating point sums, products and quotients, which every machine
//! rounds the same way. No sine, logarithm or power from the system's
//! library is used, so the same audio gives the same file everywhere.

/// The highest order of linear prediction this encoder uses. It is what a
/// FLAC stream may use at 48 kHz and below and still play everywhere.
pub(super) const MAX_LPC_ORDER: usize = 12;

/// The highest order of the fixed predictors.
pub(super) const MAX_FIXED_ORDER: usize = 4;

/// Writes to `out` what is left of `samples` after the fixed predictor of
/// `order`, from sample `order` on. The samples are of 25 bits at most, so
/// the result fits 32 with room to spare.
pub(super) fn fixed_residual(samples: &[i32], order: usize, out: &mut Vec<i32>) {
    out.clear();
    let windows = samples.windows(order + 1);
    match order {
        0 => out.extend_from_slice(samples),
        1 => out.extend(windows.map(|s| s[1] - s[0])),
        2 => out.extend(windows.map(|s| s[2] - 2 * s[1] + s[0])),
        3 => out.extend(windows.map(|s| s[3] - 3 * s[2] + 3 * s[1] - s[0])),
        _ => out.extend(windows.map(|s| s[4] - 4 * s[3] + 6 * s[2] - 4 * s[1] + s[0])),
    }
}

/// A linear predictor in the whole numbers a FLAC file stores.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Lpc {
    pub order: usize,
    /// Bits each coefficient is stored in.
    pub precision: u32,
    /// How far the sum of products is shifted down to give the prediction.
    pub shift: u32,
    /// The coefficient of the sample just before comes first.
    pub coefficients: [i32; MAX_LPC_ORDER],
}

/// The predictors of every order up to a highest one, fitted to one block.
#[derive(Debug)]
pub(super) struct Fit {
    /// `coefficients[order - 1]` holds the `order` coefficients of that
    /// order.
    coefficients: [[f64; MAX_LPC_ORDER]; MAX_LPC_ORDER],
    /// What each order leaves of the signal's power.
    error: [f64; MAX_LPC_ORDER],
    /// The highest order that was fitted.
    pub orders: usize,
}

/// The window a block is seen through while a predictor is fitted to it: a
/// parabola that is zero at both ends, so that the block's edges do not
/// count as jumps.
pub(super) fn window(length: usize, out: &mut Vec<f64>) {
    out.clear();
    let half = (length as f64 - 1.0) / 2.0;
    out.extend((0..length).map(|index| {
        let offset = if half > 0.0 {
            (index as f64 - half) / half
        } else {
            0.0
        };
        1.0 - offset * offset
    }));
}

/// Fits predictors of the orders 1 to `max_order` to `samples`. `window`
/// is as long as `samples` and `windowed` is memory to work in. Returns
/// `None` for a block that predicting cannot help: silence, or one too
/// short to tell anything from.
pub(super) fn fit(
    samples: &[i32],
    max_order: usize,
    window: &[f64],
    windowed: &mut Vec<f64>,
) -> Option<Fit> {
    let max_order = max_order.min(MAX_LPC_ORDER);
    if max_order == 0 || samples.len() <= max_order {
        return None;
    }
    windowed.clear();
    windowed.extend(
        samples
            .iter()
            .zip(window)
            .map(|(&sample, &weight)| f64::from(sample) * weight),
    );

    let mut autocorrelation = [0.0_f64; MAX_LPC_ORDER + 1];
    for (lag, sum) in autocorrelation[..=max_order].iter_mut().enumerate() {
        let pairs = windowed[lag..].iter().zip(windowed.iter());
        *sum = pairs.map(|(later, earlier)| later * earlier).sum();
    }
    if autocorrelation[0] <= 0.0 {
        return None;
    }

    // Levinson-Durbin: each order is made from the one below it.
    let mut fit = Fit {
        coefficients: [[0.0; MAX_LPC_ORDER]; MAX_LPC_ORDER],
        error: [0.0; MAX_LPC_ORDER],
        orders: 0,
    };
    let mut filter = [0.0_f64; MAX_LPC_ORDER];
    let mut error = autocorrelation[0];
    for order in 0..max_order {
        let mut reflection = -autocorrelation[order + 1];
        for tap in 0..order {
            reflection -= filter[tap] * autocorrelation[order - tap];
        }
        reflection /= error;
        filter[order] = reflection;
        for tap in 0..order / 2 {
            let low = filter[tap];
            filter[tap] += reflection * filter[order - 1 - tap];
            filter[order - 1 - tap] += reflection * low;
        }
        if order % 2 == 1 {
            filter[order / 2] += filter[order / 2] * reflection;
        }
        error *= 1.0 - reflection * reflection;
        for (stored, tap) in fit.coefficients[order].iter_mut().zip(&filter[..=order]) {
            *stored = -tap;
        }
        fit.error[order] = error;
        fit.orders = order + 1;
        if error.is_nan() || error <= 0.0 {
            // The block is predicted exactly, or the numbers ran out.
            break;
        }
    }
    Some(fit)
}

impl Fit {
    /// The order expected to give the smallest subframe for a block of
    /// `samples` samples, when storing one more order costs
    /// `bits_per_order` bits.
    pub fn likely_order(&self, samples: usize, bits_per_order: u32) -> usize {
        let scale = 0.5 / samples as f64;
        let mut best = (1, f64::INFINITY);
        for order in 1..=self.orders {
            let error = self.error[order - 1];
            // Half the logarithm of the power left over is the bits a
            // sample of the residual takes.
            let per_sample = if error > 0.0 {
                (0.5 * log2(scale * error)).max(0.0)
            } else {
                0.0
            };
            let bits =
                per_sample * (samples - order) as f64 + (order as u32 * bits_per_order) as f64;
            if bits < best.1 {
                best = (order, bits);
            }
        }
        best.0
    }

    /// The predictor of `order` with its coefficients rounded to
    /// `precision` bits. Returns `None` when they cannot be stored: a
    /// coefficient too large for the precision, or all of them zero.
    pub fn quantized(&self, order: usize, precision: u32) -> Option<Lpc> {
        let coefficients = &self.coefficients[order - 1][..order];
        let largest = coefficients.iter().fold(0.0_f64, |largest, coefficient| {
            largest.max(coefficient.abs())
        });
        if largest == 0.0 || !largest.is_finite() {
            return None;
        }
        // One bit is the sign. The shift puts the largest coefficient just
        // inside what is left.
        let magnitude_bits = precision as i32 - 1;
        let shift = magnitude_bits - exponent(largest) - 1;
        if shift < 0 {
            return None;
        }
        let shift = shift.min(15);
        let limit = (1_i64 << magnitude_bits) - 1;
        let scale = (1_i64 << shift) as f64;
        let mut lpc = Lpc {
            order,
            precision,
            shift: shift as u32,
            coefficients: [0; MAX_LPC_ORDER],
        };
        // What rounding one coefficient loses is handed to the next.
        let mut carried = 0.0;
        for (stored, &coefficient) in lpc.coefficients.iter_mut().zip(coefficients) {
            carried += coefficient * scale;
            let rounded = (carried.round() as i64).clamp(-limit - 1, limit);
            carried -= rounded as f64;
            *stored = rounded as i32;
        }
        Some(lpc)
    }
}

impl Lpc {
    /// Writes to `out` what is left of `samples` after this predictor, from
    /// sample `order` on. Returns `false`, with `out` unfinished, if a
    /// value does not fit the 32 bits a decoder works in.
    pub fn residual(&self, samples: &[i32], out: &mut Vec<i32>) -> bool {
        out.clear();
        let coefficients = &self.coefficients[..self.order];
        for window in samples.windows(self.order + 1) {
            let (history, sample) = window.split_at(self.order);
            let sum: i64 = history
                .iter()
                .rev()
                .zip(coefficients)
                .map(|(&sample, &coefficient)| i64::from(sample) * i64::from(coefficient))
                .sum();
            let left = i64::from(sample[0]) - (sum >> self.shift);
            // The lowest 32-bit number is left out as well: its sign
            // cannot be turned.
            match i32::try_from(left) {
                Ok(left) if left != i32::MIN => out.push(left),
                _ => return false,
            }
        }
        true
    }
}

/// The power of two at or below a positive number: `floor(log2(value))`.
fn exponent(value: f64) -> i32 {
    let biased = ((value.to_bits() >> 52) & 0x7FF) as i32;
    // Numbers too small to have an exponent of their own are below every
    // one that has.
    if biased == 0 { -1_023 } else { biased - 1_023 }
}

/// The base 2 logarithm of a positive number, to nine decimal places. It is
/// worked out from the number's bits and a short series, not taken from
/// the system's library, so it is the same on every machine.
fn log2(value: f64) -> f64 {
    const MANTISSA: u64 = (1 << 52) - 1;
    const ONE: u64 = 1_023 << 52;
    // 2 / ln 2.
    const SCALE: f64 = 2.885_390_081_777_926_8;
    let mantissa = f64::from_bits((value.to_bits() & MANTISSA) | ONE);
    // ln m = 2 (t + t^3 / 3 + t^5 / 5 + ...) with t = (m - 1) / (m + 1),
    // and t is at most a third.
    let t = (mantissa - 1.0) / (mantissa + 1.0);
    let square = t * t;
    let mut series = 0.0;
    for odd in [19.0, 17.0, 15.0, 13.0, 11.0, 9.0, 7.0, 5.0, 3.0, 1.0] {
        series = series * square + 1.0 / odd;
    }
    f64::from(exponent(value)) + SCALE * t * series
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(length: usize, period: f64, level: f64) -> Vec<i32> {
        (0..length)
            .map(|index| (level * (index as f64 * std::f64::consts::TAU / period).sin()) as i32)
            .collect()
    }

    #[test]
    fn the_logarithm_agrees_with_the_system_one() {
        for value in [
            1e-300, 1e-12, 0.001, 0.5, 0.75, 1.0, 1.5, 1.999_999, 2.0, 3.0, 1e6, 1.234e17, 1e300,
        ] {
            let ours = log2(value);
            assert!(
                (ours - f64::log2(value)).abs() < 1e-9,
                "log2({value}) came out as {ours}"
            );
        }
        assert_eq!(exponent(1.0), 0);
        assert_eq!(exponent(0.999), -1);
        assert_eq!(exponent(8.0), 3);
        assert_eq!(exponent(15.9), 3);
    }

    #[test]
    fn each_fixed_order_takes_a_polynomial_of_its_degree_to_nothing() {
        let cubic: Vec<i32> = (0..40_i32).map(|x| x * x * x - 7 * x * x + 5).collect();
        let mut out = Vec::new();
        for order in 0..=MAX_FIXED_ORDER {
            fixed_residual(&cubic, order, &mut out);
            assert_eq!(out.len(), cubic.len() - order);
            assert_eq!(out.iter().all(|&left| left == 0), order == 4, "{order}");
        }
        fixed_residual(&cubic, 3, &mut out);
        assert!(out.iter().all(|&left| left == 6));
    }

    #[test]
    fn a_tone_is_predicted_far_better_by_a_fitted_predictor() {
        let samples = tone(4096, 37.3, 20_000.0);
        let mut weights = Vec::new();
        window(samples.len(), &mut weights);
        assert_eq!(weights[0], 0.0);
        assert_eq!(weights[4095], 0.0);
        assert!((weights[2047] - 1.0).abs() < 1e-6);

        let fit = fit(&samples, 8, &weights, &mut Vec::new()).unwrap();
        assert_eq!(fit.orders, 8);
        let order = fit.likely_order(samples.len(), 16 + 12);
        assert!((2..=8).contains(&order));
        let lpc = fit.quantized(order, 12).unwrap();
        assert!(lpc.coefficients[..order].iter().any(|&c| c != 0));
        assert!(lpc.coefficients[order..].iter().all(|&c| c == 0));
        assert!(lpc.coefficients.iter().all(|c| (-2_048..2_048).contains(c)));

        let mut left = Vec::new();
        assert!(lpc.residual(&samples, &mut left));
        assert_eq!(left.len(), samples.len() - order);
        let mut fixed = Vec::new();
        fixed_residual(&samples, 2, &mut fixed);
        let size = |values: &[i32]| values.iter().map(|v| i64::from(v.abs())).sum::<i64>();
        assert!(size(&left) * 4 < size(&fixed));

        // The residual and the predictor give the samples back.
        let mut rebuilt = samples[..order].to_vec();
        for &left in &left {
            let sum: i64 = (0..order)
                .map(|tap| {
                    i64::from(rebuilt[rebuilt.len() - 1 - tap]) * i64::from(lpc.coefficients[tap])
                })
                .sum();
            rebuilt.push(left + (sum >> lpc.shift) as i32);
        }
        assert_eq!(rebuilt, samples);
    }

    #[test]
    fn nothing_is_fitted_to_silence_or_to_a_handful_of_samples() {
        let mut weights = Vec::new();
        window(64, &mut weights);
        assert!(fit(&[0; 64], 8, &weights, &mut Vec::new()).is_none());
        assert!(fit(&[5; 8], 8, &weights, &mut Vec::new()).is_none());
        assert!(fit(&[5; 64], 0, &weights, &mut Vec::new()).is_none());
    }

    #[test]
    fn a_residual_too_large_for_a_decoder_is_refused() {
        // A predictor that doubles the swing of full-scale 25-bit samples
        // sixteen times over.
        let lpc = Lpc {
            order: 1,
            precision: 15,
            shift: 0,
            coefficients: {
                let mut coefficients = [0; MAX_LPC_ORDER];
                coefficients[0] = -16_000;
                coefficients
            },
        };
        let samples = [16_000_000, -16_000_000, 16_000_000];
        assert!(!lpc.residual(&samples, &mut Vec::new()));
    }
}

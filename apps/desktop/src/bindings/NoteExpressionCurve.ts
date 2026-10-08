import type { NoteId } from "./NoteId";
import type { NoteCurveParameter } from "./NoteCurveParameter";
import type { NoteCurvePoint } from "./NoteCurvePoint";
export type NoteExpressionCurve = { note: NoteId, parameter: NoteCurveParameter, points: NoteCurvePoint[] };

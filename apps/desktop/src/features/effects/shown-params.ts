import {
  readParam,
  writeParam,
  type ParamBinder,
  type ParamDescriptor,
} from "@/features/params"

/**
 * The settings as the controls show them: the stored ones, with whatever a
 * control being dragged holds laid over them. A display drawn from these
 * follows the pointer and does not wait for the project to answer. While
 * nothing is held this is `params` itself.
 */
export function shownParams<Params extends object>(
  descriptor: ParamDescriptor<Params>,
  params: Params,
  bind: ParamBinder
): Params {
  let shown = params
  for (const info of descriptor.params) {
    const value = bind.value(info.id)
    if (value !== readParam(shown, info)) {
      shown = writeParam(shown, info, value)
    }
  }
  return shown
}

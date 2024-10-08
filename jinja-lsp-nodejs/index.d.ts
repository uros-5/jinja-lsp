/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export function basic(content: string): number | null
export interface JsPosition {
  line: number
  character: number
}
export const enum JsIdentifierType {
  ForLoopKey = 0,
  ForLoopValue = 1,
  ForLoopCount = 2,
  SetVariable = 3,
  WithVariable = 4,
  MacroName = 5,
  MacroParameter = 6,
  TemplateBlock = 7,
  BackendVariable = 8,
  UndefinedVariable = 9,
  JinjaTemplate = 10,
  Link = 11
}
export interface JsIdentifier {
  start: JsPosition
  end: JsPosition
  name: string
  identifierType: JsIdentifierType
  error?: string
}
export interface JsHover {
  kind: string
  value: string
  range?: JsRange
  label?: string
  documentaion?: string
}
export interface JsRange {
  start: JsPosition
  end: JsPosition
}
export interface JsLocation {
  uri: string
  range: JsRange
  isBackend: boolean
  name: string
}
export interface JsCompletionItem {
  completionType: JsCompletionType
  label: string
  kind: Kind2
  description: string
  newText?: string
  insert?: JsRange
  replace?: JsRange
}
export const enum Kind2 {
  VARIABLE = 0,
  FIELD = 1,
  FUNCTION = 2,
  MODULE = 3,
  CONSTANT = 4,
  FILE = 5,
  TEXT = 6
}
export const enum JsCompletionType {
  Filter = 0,
  Identifier = 1,
  Snippets = 2
}
export interface Action {
  name: string
  description: string
}
export class NodejsLspFiles {
  constructor()
  /** Actions can come from unsaved context. */
  addLinkHints(uri: string, actions?: Array<Action> | undefined | null): void
  saveLinkHint(actions?: Array<Action> | undefined | null, hint?: string | undefined | null): void
  removeTempLinkHint(hint?: string | undefined | null): void
  deleteAll(filename: string): void
  addOne(id: number, filename: string, content: string, line: number, ext: string, col?: number | undefined | null): Array<JsIdentifier>
  getVariables(id: string, line: number): Array<JsIdentifier> | null
  hover(id: number, filename: string, line: number, position: JsPosition): JsHover | null
  complete(id: number, filename: string, line: number, position: JsPosition): Array<JsCompletionItem> | null
  gotoDefinition(id: number, filename: string, line: number, position: JsPosition): Array<JsLocation> | null
}

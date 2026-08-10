import type { RdpInputEvent } from "./rdpInput";

const PHYSICAL_TEXT_KEYS = new Set([
  "Backspace",
  "Enter",
  "Tab",
  "Escape",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "ArrowDown",
]);

export function shouldUsePhysicalRdpKey(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey">,
) {
  return event.ctrlKey || event.altKey || event.metaKey || PHYSICAL_TEXT_KEYS.has(event.key);
}

export function buildRdpUnicodeInput(text: string): RdpInputEvent[] {
  return text.length > 0 ? [{ type: "unicode", text }] : [];
}

export function rdpBeforeInputText(
  event: Pick<InputEvent, "data" | "inputType" | "isComposing">,
): string | null {
  if (event.isComposing) return null;
  if (event.inputType !== "insertText" && event.inputType !== "insertCompositionText") return null;
  return event.data || null;
}

export function rdpCompositionCommitText(text: string): string | null {
  return text.length > 0 ? text : null;
}

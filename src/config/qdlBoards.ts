/** i18n key for the EDL-entry hint, from the backend-resolved method (defaults to jumper). */
export function qdlInstructionsKey(edlEntry?: string | null): string {
  return edlEntry === 'button' ? 'device.qdlInstructionsButton' : 'device.qdlInstructions';
}

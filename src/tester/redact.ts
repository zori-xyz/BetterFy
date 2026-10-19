// The first pass over anything the tester journal keeps; the desktop engine
// applies the same rules again before a report leaves the device
// (src-tauri/src/tester_reports.rs). A Windows profile folder may contain
// spaces, so its whole name is removed.

const rules: [RegExp, string][] = [
  [/\b[a-z]:[\\/]+users[\\/]+[^\\/"\r\n]+/gi, "%USERPROFILE%"],
  [/\/(?:Users|home)\/[^/"\r\n]+/g, "~"],
  [/[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}/g, "•••@•••"],
  [/\d{9,}/g, "•••"],
];

export function redactText(text: string) {
  return rules.reduce((value, [pattern, replacement]) => value.replace(pattern, replacement), text);
}

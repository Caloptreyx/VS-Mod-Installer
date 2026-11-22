import { DISABLED_SUFFIX } from './constants.ts';

/** The file name of a mod once disabled: the game skips files that don't end in `.zip`, `.dll` or `.cs`. */
export const disabledName = (fileName: string) => `${fileName}${DISABLED_SUFFIX}`;

/** The file name of a disabled mod once enabled again. */
export const enabledName = (fileName: string) =>
  fileName.toLowerCase().endsWith(DISABLED_SUFFIX) ? fileName.slice(0, -DISABLED_SUFFIX.length) : fileName;

/**
 * The name a release is installed under: its original file name, with `.disabled` when it
 * replaces a disabled file, so an update keeps the mod's enabled state.
 */
export const installedName = (releaseFileName: string, replacingEnabled: boolean | null) =>
  replacingEnabled === false ? disabledName(releaseFileName) : releaseFileName;

/** The root the panel's file endpoints take for the mods folder (`data/Mods` -> `/data/Mods`). */
export const modsRoot = (modsDir: string) => `/${modsDir}`;
/** Splits a folder path into its parent (as a file endpoint root) and its name. */
export const parentAndName = (path: string): [string, string] => {
  const index = path.lastIndexOf('/');
  return index < 0 ? ['/', path] : [`/${path.slice(0, index)}`, path.slice(index + 1)];
};

export const formatCount = (value: number): string =>
  new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 }).format(value);

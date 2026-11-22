export const EXTENSION_ID = 'dev.caloptreyx.vsmodinstaller';
export const API_PREFIX = 'vs-mod-installer';

/** The ModDB, for links to mod pages. */
export const MODDB_URL = 'https://mods.vintagestory.at';

/** Vintage Story only loads `.zip`, `.dll` and `.cs` files, so a renamed file is ignored. */
export const DISABLED_SUFFIX = '.disabled';

export const modInstallerQueryKey = (...parts: unknown[]) => ['extensions', EXTENSION_ID, ...parts] as const;

export const serverModsPath = (uuidShort: number | string, path = '') => `/server/${uuidShort}/vs-mods${path}`;

/** The ModDB page of a mod. */
export const moddbPageUrl = (mod: { urlalias: string | null; assetid?: number; modid: number }) =>
  mod.urlalias ? `${MODDB_URL}/${mod.urlalias}` : `${MODDB_URL}/show/mod/${mod.assetid ?? mod.modid}`;

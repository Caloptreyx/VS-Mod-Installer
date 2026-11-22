import { axiosInstance } from '@/api/axios.ts';
import createDirectory from '@/api/server/files/createDirectory.ts';
import deleteFiles from '@/api/server/files/deleteFiles.ts';
import renameFiles from '@/api/server/files/renameFiles.ts';
import type { InstalledMod, Release, ServerEnvironment } from './schemas.ts';
import { disabledName, enabledName, installedName, modsRoot, parentAndName } from './files.ts';

/**
 * Everything that changes files goes through the panel's own file endpoints, so the user's file
 * permissions, subuser ignore rules and the server activity log apply as for any file change.
 */

async function ensureModsDir(serverUuid: string, environment: ServerEnvironment): Promise<void> {
  if (environment.modsDirExists) return;
  const [parent, name] = parentAndName(environment.modsDir);
  // the folder may exist by now (detection is cached briefly); the download reports real problems
  await createDirectory(serverUuid, parent, name).catch(() => undefined);
}

/**
 * Downloads `release` into the mods folder under its original file name. Wings fetches the file
 * from the ModDB's CDN itself; the request returns once the file is written. When `replacing` is
 * given (an update or a version change), the new file keeps the old file's enabled state and the
 * old file is removed only after the download finished, so a failed download never leaves the
 * server without the mod.
 */
export async function installRelease(
  serverUuid: string,
  environment: ServerEnvironment,
  release: Release,
  replacing: InstalledMod | null,
): Promise<string> {
  if (!release.installable) throw new Error(`Release ${release.modVersion} has no file that can be installed.`);

  await ensureModsDir(serverUuid, environment);
  const target = installedName(release.filename, replacing ? replacing.enabled : null);
  await axiosInstance.post(`/api/client/servers/${serverUuid}/files/pull`, {
    root: modsRoot(environment.modsDir),
    url: release.mainfile,
    name: target,
    foreground: true,
  });

  if (replacing && replacing.fileName !== target) {
    await deleteFiles(serverUuid, modsRoot(environment.modsDir), [replacing.fileName]);
  }

  return target;
}

export async function setModEnabled(
  serverUuid: string,
  modsDir: string,
  mod: InstalledMod,
  enabled: boolean,
): Promise<void> {
  const to = enabled ? enabledName(mod.fileName) : disabledName(mod.fileName);
  await renameFiles({ uuid: serverUuid, root: modsRoot(modsDir), files: [{ from: mod.fileName, to }] });
}

export async function deleteMod(serverUuid: string, modsDir: string, mod: InstalledMod): Promise<void> {
  await deleteFiles(serverUuid, modsRoot(modsDir), [mod.fileName]);
}

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import { useServerStore } from '@/stores/server.ts';
import type { InstalledMod, ModUpdate } from '../lib/schemas.ts';
import { getInstalledMods, getModUpdates } from '../api.ts';
import { modInstallerQueryKey } from '../lib/constants.ts';

/**
 * Mods in the server's mods folder with their `modinfo.json` and ModDB page, plus the updates
 * the ModDB has for them (checked once the list is loaded).
 */
export function useInstalledMods({ enabled = true, withUpdates = false } = {}) {
  const serverUuid = useServerStore((state) => state.server.uuid);
  const queryClient = useQueryClient();
  const modsKey = modInstallerQueryKey('mods', serverUuid);

  const list = useQuery({
    queryKey: modsKey,
    queryFn: () => getInstalledMods(serverUuid),
    enabled,
  });

  const hasModDbMods = !!list.data?.mods.some((mod) => mod.moddb);
  const updates = useQuery({
    queryKey: modInstallerQueryKey('updates', serverUuid, list.dataUpdatedAt),
    queryFn: () => getModUpdates(serverUuid),
    enabled: enabled && withUpdates && hasModDbMods,
    staleTime: 5 * 60 * 1000,
  });

  const maps = useMemo(() => {
    const byModDbId = new Map<number, InstalledMod>();
    const byModId = new Map<string, InstalledMod>();
    for (const mod of list.data?.mods ?? []) {
      // an enabled file wins over a disabled copy of the same mod
      if (mod.moddb && (!byModDbId.has(mod.moddb.modid) || mod.enabled)) byModDbId.set(mod.moddb.modid, mod);
      if (mod.info && (!byModId.has(mod.info.modid) || mod.enabled)) byModId.set(mod.info.modid, mod);
    }
    const updatesByFile = new Map<string, ModUpdate>((updates.data ?? []).map((update) => [update.fileName, update]));

    return { byModDbId, byModId, updatesByFile };
  }, [list.data, updates.data]);

  return {
    mods: list.data?.mods,
    modsDir: list.data?.modsDir,
    catalogError: list.data?.catalogError ?? null,
    ...maps,
    loading: list.isLoading,
    error: list.error ? httpErrorToHuman(list.error) : null,
    refreshing: list.isFetching,
    checkingUpdates: updates.isFetching,
    updatesError: updates.error ? httpErrorToHuman(updates.error) : null,
    refresh: () => queryClient.invalidateQueries({ queryKey: modsKey }),
  };
}

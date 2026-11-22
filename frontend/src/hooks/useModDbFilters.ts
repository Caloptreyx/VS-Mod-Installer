import { useQuery } from '@tanstack/react-query';
import { useServerStore } from '@/stores/server.ts';
import { getModDbFilters } from '../api.ts';
import { modInstallerQueryKey } from '../lib/constants.ts';

/** Tags in use on the ModDB and every game version, newest first. */
export function useModDbFilters(enabled = true) {
  const serverUuid = useServerStore((state) => state.server.uuid);

  const { data } = useQuery({
    queryKey: modInstallerQueryKey('filters'),
    queryFn: () => getModDbFilters(serverUuid),
    staleTime: 30 * 60 * 1000,
    enabled,
  });

  return { tags: data?.tags ?? [], gameVersions: data?.gameVersions ?? [] };
}

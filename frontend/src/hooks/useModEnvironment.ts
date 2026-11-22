import { useQuery } from '@tanstack/react-query';
import { httpErrorToHuman } from '@/api/axios.ts';
import { useServerStore } from '@/stores/server.ts';
import { getServerEnvironment } from '../api.ts';
import { modInstallerQueryKey } from '../lib/constants.ts';

/** Whether the server runs Vintage Story, its mods folder and game version. */
export function useModEnvironment() {
  const serverUuid = useServerStore((state) => state.server.uuid);

  const { data, isLoading, error } = useQuery({
    queryKey: modInstallerQueryKey('environment', serverUuid),
    queryFn: () => getServerEnvironment(serverUuid),
    staleTime: 30 * 1000,
  });

  return { environment: data, loading: isLoading, error: error ? httpErrorToHuman(error) : null };
}

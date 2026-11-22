import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { API_PREFIX, EXTENSION_ID } from './lib/constants.ts';
import {
  type DependencyStatus,
  dependencyStatusSchema,
  type InstalledMod,
  installedModSchema,
  type ModDetail,
  type ModInfo,
  type ModSummary,
  type ModUpdate,
  type Release,
  type ServerEnvironment,
  modDetailSchema,
  modInfoSchema,
  modSummarySchema,
  modUpdateSchema,
  releaseSchema,
  serverEnvironmentSchema,
} from './lib/schemas.ts';

const base = (serverUuid: string) => `/api/client/servers/${serverUuid}/${API_PREFIX}`;

export const getServerEnvironment = async (serverUuid: string): Promise<ServerEnvironment> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/environment`);
  return parseFromApi(serverEnvironmentSchema, data.environment);
};

export type SortOption = 'relevance' | 'downloads' | 'follows' | 'trending' | 'updated' | 'newest';

export interface SearchParams {
  text: string;
  tag: string | null;
  gameVersion: string | null;
  includeClient: boolean;
  sort: SortOption;
  page: number;
  perPage: number;
}

export interface SearchResult {
  mods: ModSummary[];
  total: number;
}

export const searchMods = async (serverUuid: string, params: SearchParams): Promise<SearchResult> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/moddb/search`, {
    params: {
      text: params.text,
      tag: params.tag ?? undefined,
      game_version: params.gameVersion ?? undefined,
      include_client: params.includeClient,
      sort: params.sort,
      page: params.page,
      per_page: params.perPage,
    },
  });
  return {
    mods: data.mods.map((mod: unknown) => parseFromApi(modSummarySchema, mod)),
    total: data.total,
  };
};

export interface ModDbFilters {
  tags: { name: string; count: number }[];
  gameVersions: string[];
}

export const getModDbFilters = async (serverUuid: string): Promise<ModDbFilters> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/moddb/filters`);
  return { tags: data.tags, gameVersions: data.game_versions };
};

export interface ModDetailResult {
  mod: ModDetail;
  releases: Release[];
  recommendedRelease: number | null;
  gameVersion: string | null;
}

export const getModDetail = async (
  serverUuid: string,
  id: string | number,
  gameVersion: string | null,
): Promise<ModDetailResult> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/moddb/mod`, {
    params: { id, game_version: gameVersion ?? undefined },
  });
  return {
    mod: parseFromApi(modDetailSchema, data.mod),
    releases: data.releases.map((release: unknown) => parseFromApi(releaseSchema, release)),
    recommendedRelease: data.recommended_release ?? null,
    gameVersion: data.game_version ?? null,
  };
};

export interface InstalledModsResult {
  modsDir: string;
  exists: boolean;
  mods: InstalledMod[];
  catalogError: string | null;
}

export const getInstalledMods = async (serverUuid: string): Promise<InstalledModsResult> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/mods`);
  return {
    modsDir: data.mods_dir,
    exists: data.exists,
    mods: data.mods.map((mod: unknown) => parseFromApi(installedModSchema, mod)),
    catalogError: data.catalog_error ?? null,
  };
};

export const getModUpdates = async (serverUuid: string): Promise<ModUpdate[]> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/mods/updates`);
  return data.updates.map((update: unknown) => parseFromApi(modUpdateSchema, update));
};

export interface DependenciesResult {
  fileName: string;
  info: ModInfo | null;
  dependencies: DependencyStatus[];
}

export const getDependencies = async (serverUuid: string, file: string): Promise<DependenciesResult> => {
  const { data } = await axiosInstance.get(`${base(serverUuid)}/mods/dependencies`, { params: { file } });
  return {
    fileName: data.file_name,
    info: data.info ? parseFromApi(modInfoSchema, data.info) : null,
    dependencies: data.dependencies.map((dependency: unknown) => parseFromApi(dependencyStatusSchema, dependency)),
  };
};

/** Admin settings, in the backend's field names. */
export interface Settings {
  eggs: string[];
  max_archive_mb: number;
}

const adminBase = `/api/admin/extensions/${EXTENSION_ID}`;

export const getSettings = async (): Promise<Settings> =>
  (await axiosInstance.get(`${adminBase}/settings`)).data.settings;

export const updateSettings = async (settings: Settings): Promise<Settings> =>
  (await axiosInstance.put(`${adminBase}/settings`, settings)).data.settings;

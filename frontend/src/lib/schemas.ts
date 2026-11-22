import { z } from 'zod';

export const detectionSourceSchema = z.enum([
  'settings',
  'egg',
  'startup',
  'config',
  'default',
  'files',
  'log',
  'variables',
]);
export type DetectionSource = z.infer<typeof detectionSourceSchema>;

export const serverEnvironmentSchema = z.object({
  supported: z.boolean(),
  detectedBy: detectionSourceSchema.nullable(),
  dataPath: z.string(),
  dataPathSource: detectionSourceSchema,
  modsDir: z.string(),
  modsDirSource: detectionSourceSchema,
  modsDirExists: z.boolean(),
  gameVersion: z.string().nullable(),
  gameVersionSource: detectionSourceSchema.nullable(),
  canReadContent: z.boolean(),
});
export type ServerEnvironment = z.infer<typeof serverEnvironmentSchema>;

export const sideSchema = z.enum(['client', 'server', 'both']);
export type ModSide = z.infer<typeof sideSchema>;

export const modSummarySchema = z.object({
  modid: z.number(),
  assetid: z.number(),
  name: z.string(),
  summary: z.string(),
  author: z.string(),
  urlalias: z.string().nullable(),
  side: sideSchema,
  kind: z.string(),
  logo: z.string().nullable(),
  tags: z.array(z.string()),
  modidstrs: z.array(z.string()),
  downloads: z.number(),
  follows: z.number(),
  trending: z.number(),
  comments: z.number(),
  lastReleased: z.string().nullable(),
});
export type ModSummary = z.infer<typeof modSummarySchema>;

export const compatibilitySchema = z.enum(['incompatible', 'unknown', 'same_major', 'exact']);
export type Compatibility = z.infer<typeof compatibilitySchema>;

export const releaseSchema = z.object({
  releaseId: z.number(),
  mainfile: z.string(),
  filename: z.string(),
  downloads: z.number(),
  gameVersions: z.array(z.string()),
  modidstr: z.string(),
  modVersion: z.string(),
  created: z.string().nullable(),
  changelog: z.string().nullable(),
  compatibility: compatibilitySchema,
  prerelease: z.boolean(),
  installable: z.boolean(),
});
export type Release = z.infer<typeof releaseSchema>;

export const modDetailSchema = z.object({
  modid: z.number(),
  assetid: z.number(),
  name: z.string(),
  text: z.string(),
  author: z.string(),
  urlalias: z.string().nullable(),
  logo: z.string().nullable(),
  links: z.object({
    homepage: z.string().nullable(),
    source: z.string().nullable(),
    issues: z.string().nullable(),
    wiki: z.string().nullable(),
    trailer: z.string().nullable(),
  }),
  downloads: z.number(),
  follows: z.number(),
  comments: z.number(),
  side: sideSchema,
  kind: z.string(),
  created: z.string().nullable(),
  lastReleased: z.string().nullable(),
  tags: z.array(z.string()),
  screenshots: z.array(z.object({ url: z.string(), thumbnail: z.string().nullable() })),
});
export type ModDetail = z.infer<typeof modDetailSchema>;

export const modRefSchema = z.object({
  modid: z.number(),
  assetid: z.number(),
  name: z.string(),
  summary: z.string().nullable(),
  logo: z.string().nullable(),
  urlalias: z.string().nullable(),
  side: sideSchema,
});
export type ModRef = z.infer<typeof modRefSchema>;

export const dependencySchema = z.object({
  modid: z.string(),
  version: z.string(),
});

export const modInfoSchema = z.object({
  modid: z.string(),
  name: z.string(),
  version: z.string(),
  kind: z.string().nullable(),
  side: z.string(),
  description: z.string().nullable(),
  authors: z.array(z.string()),
  dependencies: z.array(dependencySchema),
});
export type ModInfo = z.infer<typeof modInfoSchema>;

export const infoErrorSchema = z.enum([
  'permission',
  'code_mod',
  'no_modinfo',
  'invalid_modinfo',
  'invalid_archive',
  'too_large',
  'unreadable',
]);
export type InfoError = z.infer<typeof infoErrorSchema>;

export const installedModSchema = z.object({
  fileName: z.string(),
  enabled: z.boolean(),
  kind: z.enum(['zip', 'dll', 'cs', 'folder']),
  size: z.number(),
  modified: z.string(),
  info: modInfoSchema.nullable(),
  infoError: infoErrorSchema.nullable(),
  moddb: modRefSchema.nullable(),
  unmetDependencies: z.array(z.string()),
});
export type InstalledMod = z.infer<typeof installedModSchema>;

export const modUpdateSchema = z.object({
  fileName: z.string(),
  modid: z.number(),
  release: releaseSchema,
});
export type ModUpdate = z.infer<typeof modUpdateSchema>;

export const dependencyStateSchema = z.enum([
  'satisfied',
  'builtin',
  'game_too_old',
  'disabled',
  'outdated',
  'missing',
]);
export type DependencyState = z.infer<typeof dependencyStateSchema>;

export const dependencyProblemSchema = z.enum(['not_on_moddb', 'no_release', 'no_new_enough_release']);
export type DependencyProblem = z.infer<typeof dependencyProblemSchema>;

export const dependencyStatusSchema = z.object({
  modid: z.string(),
  requiredVersion: z.string(),
  state: dependencyStateSchema,
  installedVersion: z.string().nullable(),
  moddb: modRefSchema.nullable(),
  release: releaseSchema.nullable(),
  problem: dependencyProblemSchema.nullable(),
});
export type DependencyStatus = z.infer<typeof dependencyStatusSchema>;

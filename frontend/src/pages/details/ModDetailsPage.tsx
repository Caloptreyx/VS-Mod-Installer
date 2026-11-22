import {
  faArrowLeft,
  faArrowUpRightFromSquare,
  faBook,
  faBug,
  faCheck,
  faCode,
  faDownload,
  faHouse,
  faRightLeft,
  faVideo,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { SimpleGrid } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Divider from '@/elements/layout/Divider.tsx';
import Group from '@/elements/layout/Group.tsx';
import Paper from '@/elements/layout/Paper.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Anchor from '@/elements/typography/Anchor.tsx';
import Text from '@/elements/typography/Text.tsx';
import Title from '@/elements/typography/Title.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import type { Release } from '../../lib/schemas.ts';
import { getModDetail } from '../../api.ts';
import InstallModModal from '../../components/InstallModModal.tsx';
import { SideBadge } from '../../components/ModBadges.tsx';
import ModDescription from '../../components/ModDescription.tsx';
import { useInstalledMods } from '../../hooks/useInstalledMods.ts';
import { useModEnvironment } from '../../hooks/useModEnvironment.ts';
import { moddbPageUrl, modInstallerQueryKey, serverModsPath } from '../../lib/constants.ts';
import { formatCount } from '../../lib/files.ts';
import { useExtTranslations } from '../../translations.ts';
import ModReleasesTab from './ModReleasesTab.tsx';

export default function ModDetailsPage() {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const { modId } = useParams<{ modId: string }>();
  const { uuid: serverUuid, uuidShort } = useServerStore((state) => state.server);
  const canInstall = useServerCan('files.create');
  const canChange = useServerCan(['files.create', 'files.delete'], false);

  const { environment, loading: environmentLoading } = useModEnvironment();
  const gameVersion = environment?.gameVersion ?? null;
  const { byModDbId, byModId, updatesByFile } = useInstalledMods({
    enabled: !!environment?.supported,
    withUpdates: true,
  });

  const detail = useQuery({
    queryKey: modInstallerQueryKey('mod', modId, gameVersion),
    queryFn: () => getModDetail(serverUuid, modId!, gameVersion),
    enabled: !!modId && !environmentLoading && !!environment?.supported,
  });

  const [tab, setTab] = useState<string | null>('description');
  const [installOpen, setInstallOpen] = useState(false);
  const [installReleaseId, setInstallReleaseId] = useState<number | null>(null);

  const goBack = () => {
    if ((window.history.state?.idx ?? 0) > 0) {
      navigate(-1);
    } else {
      navigate(serverModsPath(uuidShort, '/browse'));
    }
  };

  const openInstall = (release: Release | null) => {
    setInstallReleaseId(release?.releaseId ?? null);
    setInstallOpen(true);
  };

  const data = detail.data;
  const mod = data?.mod;
  const installed = mod
    ? (byModDbId.get(mod.modid) ??
      (data?.releases ?? []).map((release) => byModId.get(release.modidstr)).find(Boolean) ??
      null)
    : null;
  const update = installed ? updatesByFile.get(installed.fileName) : undefined;

  const links = mod
    ? [
        { href: mod.links.homepage, icon: faHouse, label: tExt('pages.server.vsMods.details.links.homepage', {}) },
        { href: mod.links.source, icon: faCode, label: tExt('pages.server.vsMods.details.links.source', {}) },
        { href: mod.links.issues, icon: faBug, label: tExt('pages.server.vsMods.details.links.issues', {}) },
        { href: mod.links.wiki, icon: faBook, label: tExt('pages.server.vsMods.details.links.wiki', {}) },
        { href: mod.links.trailer, icon: faVideo, label: tExt('pages.server.vsMods.details.links.trailer', {}) },
      ].filter((link) => !!link.href)
    : [];

  return (
    <ServerContentContainer title={mod?.name ?? tExt('pages.server.vsMods.title', {})} hideTitleComponent>
      <Button variant='subtle' leftSection={<FontAwesomeIcon icon={faArrowLeft} />} onClick={goBack} mb='sm'>
        {tExt('pages.server.vsMods.button.back', {})}
      </Button>

      {environment && !environment.supported ? (
        <Alert color='yellow' title={tExt('pages.server.vsMods.environment.alert.unsupportedTitle', {})}>
          {tExt('pages.server.vsMods.environment.alert.unsupportedContent', {})}
        </Alert>
      ) : detail.error ? (
        <Alert color='red' title={tExt('pages.server.vsMods.details.notFound', {})}>
          {httpErrorToHuman(detail.error)}
        </Alert>
      ) : !data || !mod ? (
        <Spinner.Centered />
      ) : (
        <>
          <InstallModModal
            opened={installOpen}
            onClose={() => setInstallOpen(false)}
            mod={{ modid: mod.modid, name: mod.name, logo: mod.logo, side: mod.side }}
            installed={installed}
            initialReleaseId={installReleaseId}
          />

          <Paper withBorder p='lg' radius='md' mb='md'>
            <Group justify='space-between' align='flex-start' gap='lg'>
              <Group align='flex-start' wrap='nowrap' gap='lg' style={{ flex: 1, minWidth: 0 }}>
                <Avatar src={mod.logo} name={mod.name} size={96} radius='md' />
                <Stack gap={6} style={{ minWidth: 0 }}>
                  <Group gap='sm'>
                    <Title order={2}>{mod.name}</Title>
                    {installed && (
                      <Badge color='green' variant='light' leftSection={<FontAwesomeIcon icon={faCheck} />}>
                        {tExt(
                          installed.enabled
                            ? 'pages.server.vsMods.details.installedVersion'
                            : 'pages.server.vsMods.details.installedDisabled',
                          { version: installed.info?.version ?? installed.fileName },
                        )}
                      </Badge>
                    )}
                    {update && (
                      <Badge color='blue' variant='light'>
                        {tExt('pages.server.vsMods.installed.updateAvailable', { version: update.release.modVersion })}
                      </Badge>
                    )}
                  </Group>
                  <Text size='sm' c='dimmed'>
                    {tExt('pages.server.vsMods.browse.by', { author: mod.author })}
                  </Text>
                  <Group gap={6}>
                    <SideBadge side={mod.side} />
                    {mod.tags.map((tag) => (
                      <Badge key={tag} variant='default'>
                        {tag}
                      </Badge>
                    ))}
                  </Group>
                </Stack>
              </Group>

              <Stack gap='xs' align='stretch'>
                {installed
                  ? canChange && (
                      <Button leftSection={<FontAwesomeIcon icon={faRightLeft} />} onClick={() => openInstall(null)}>
                        {update
                          ? tExt('pages.server.vsMods.button.update', {})
                          : tExt('pages.server.vsMods.button.changeVersion', {})}
                      </Button>
                    )
                  : canInstall && (
                      <Button leftSection={<FontAwesomeIcon icon={faDownload} />} onClick={() => openInstall(null)}>
                        {tExt('pages.server.vsMods.button.install', {})}
                      </Button>
                    )}
                <Button
                  variant='default'
                  onClick={() => {
                    window.open(moddbPageUrl(mod), '_blank', 'noopener,noreferrer');
                  }}
                  leftSection={<FontAwesomeIcon icon={faArrowUpRightFromSquare} />}
                >
                  {tExt('pages.server.vsMods.button.openModDb', {})}
                </Button>
              </Stack>
            </Group>

            <Divider my='md' />

            <Group justify='space-between' gap='lg'>
              <Group gap='xl'>
                <Stat
                  label={tExt('pages.server.vsMods.details.stats.downloads', {})}
                  value={formatCount(mod.downloads)}
                />
                <Stat
                  label={tExt('pages.server.vsMods.details.stats.followers', {})}
                  value={formatCount(mod.follows)}
                />
                {mod.lastReleased && (
                  <Stat
                    label={tExt('pages.server.vsMods.details.stats.updated', {})}
                    value={<FormattedTimestamp timestamp={mod.lastReleased} />}
                  />
                )}
                {mod.created && (
                  <Stat
                    label={tExt('pages.server.vsMods.details.stats.created', {})}
                    value={<FormattedTimestamp timestamp={mod.created} />}
                  />
                )}
              </Group>
              {links.length > 0 && (
                <Group gap='md'>
                  {links.map((link) => (
                    <Anchor key={link.label} href={link.href!} target='_blank' rel='noopener noreferrer' size='sm'>
                      <FontAwesomeIcon icon={link.icon} /> {link.label}
                    </Anchor>
                  ))}
                </Group>
              )}
            </Group>
          </Paper>

          <Tabs value={tab} onChange={setTab}>
            <Tabs.List>
              <Tabs.Tab value='description'>{tExt('pages.server.vsMods.details.tabs.description', {})}</Tabs.Tab>
              <Tabs.Tab value='releases'>
                {tExt('pages.server.vsMods.details.tabs.releases', {})}
                <Badge ml='xs' variant='default' size='sm'>
                  {data.releases.length}
                </Badge>
              </Tabs.Tab>
              {mod.screenshots.length > 0 && (
                <Tabs.Tab value='screenshots'>{tExt('pages.server.vsMods.details.tabs.screenshots', {})}</Tabs.Tab>
              )}
            </Tabs.List>

            <Tabs.Panel value='description' pt='md'>
              <Paper withBorder p='lg' radius='md'>
                <ModDescription html={mod.text} />
              </Paper>
            </Tabs.Panel>

            <Tabs.Panel value='releases' pt='md'>
              <ModReleasesTab
                releases={data.releases}
                recommendedRelease={data.recommendedRelease}
                gameVersion={gameVersion}
                installed={installed}
                canInstall={installed ? canChange : canInstall}
                onInstall={openInstall}
              />
            </Tabs.Panel>

            {mod.screenshots.length > 0 && (
              <Tabs.Panel value='screenshots' pt='md'>
                <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing='md'>
                  {mod.screenshots.map((screenshot) => (
                    <a key={screenshot.url} href={screenshot.url} target='_blank' rel='noopener noreferrer'>
                      <img
                        src={screenshot.url}
                        alt=''
                        loading='lazy'
                        referrerPolicy='no-referrer'
                        style={{ width: '100%', borderRadius: 'var(--mantine-radius-md)', display: 'block' }}
                      />
                    </a>
                  ))}
                </SimpleGrid>
              </Tabs.Panel>
            )}
          </Tabs>
        </>
      )}
    </ServerContentContainer>
  );
}

function Stat({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <Stack gap={0}>
      <Text size='xs' c='dimmed' tt='uppercase' fw={600}>
        {label}
      </Text>
      <Text fw={600}>{value}</Text>
    </Stack>
  );
}

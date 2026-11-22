import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { ReactNode } from 'react';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Group from '@/elements/layout/Group.tsx';
import { useModEnvironment } from '../hooks/useModEnvironment.ts';
import { useExtTranslations } from '../translations.ts';
import EnvironmentAlert from './EnvironmentAlert.tsx';
import EnvironmentBadges from './EnvironmentBadges.tsx';
import ModsSubNavigation from './ModsSubNavigation.tsx';

/**
 * Shared chrome of the Installed and Browse pages: title, detection badges, tabs and alerts.
 * On a server that isn't a Vintage Story server only an explanation is shown.
 */
export default function ModsPageLayout({ contentRight, children }: { contentRight?: ReactNode; children: ReactNode }) {
  const { t: tExt } = useExtTranslations();
  const { environment, loading, error } = useModEnvironment();

  return (
    <ServerContentContainer
      title={tExt('pages.server.vsMods.title', {})}
      contentRight={
        <Group gap='sm'>
          <EnvironmentBadges environment={environment} loading={loading} />
          {environment?.supported && contentRight}
        </Group>
      }
    >
      {error ? (
        <Alert color='red'>{error}</Alert>
      ) : loading || !environment ? (
        <Spinner.Centered />
      ) : !environment.supported ? (
        <EmptyState
          icon={faCubes}
          title={tExt('pages.server.vsMods.environment.alert.unsupportedTitle', {})}
          description={tExt('pages.server.vsMods.environment.alert.unsupportedContent', {})}
        />
      ) : (
        <>
          <ModsSubNavigation />
          <EnvironmentAlert environment={environment} />
          {children}
        </>
      )}
    </ServerContentContainer>
  );
}

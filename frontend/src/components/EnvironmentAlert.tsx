import { faLock, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Alert from '@/elements/feedback/Alert.tsx';
import type { ServerEnvironment } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

/** Explains what the page can't do: a game version that wasn't found, or missing permissions. */
export default function EnvironmentAlert({ environment }: { environment: ServerEnvironment | undefined }) {
  const { t: tExt } = useExtTranslations();

  if (!environment?.supported) return null;

  if (!environment.canReadContent) {
    return (
      <Alert
        color='yellow'
        icon={<FontAwesomeIcon icon={faLock} />}
        title={tExt('pages.server.vsMods.environment.alert.noContentTitle', {})}
        mb='md'
      >
        {tExt('pages.server.vsMods.environment.alert.noContentContent', {})}
      </Alert>
    );
  }

  if (environment.gameVersion) return null;

  return (
    <Alert
      color='yellow'
      icon={<FontAwesomeIcon icon={faTriangleExclamation} />}
      title={tExt('pages.server.vsMods.environment.alert.versionTitle', {})}
      mb='md'
    >
      {tExt('pages.server.vsMods.environment.alert.versionContent', {})}
    </Alert>
  );
}

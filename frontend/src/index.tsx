import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import AdminConfigurationPage from './pages/admin/AdminConfigurationPage.tsx';
import BrowseModsPage from './pages/browse/BrowseModsPage.tsx';
import ModDetailsPage from './pages/details/ModDetailsPage.tsx';
import InstalledModsPage from './pages/installed/InstalledModsPage.tsx';
import { getExtTranslations } from './translations.ts';

// The panel's file permissions decide everything: listing the mods folder needs files.read.
const PERMISSION = 'files.read';

class CaloptreyxVsModInstallerExtension extends Extension {
  public cardConfigurationPage: React.FC | null = AdminConfigurationPage;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faCubes} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes
        .addServerRoute({
          name: () => getExtTranslations().t('pages.server.vsMods.title', {}),
          icon: faCubes,
          path: '/vs-mods',
          element: InstalledModsPage,
          permission: PERMISSION,
        })
        .addServerRoute({
          name: undefined,
          path: '/vs-mods/browse',
          element: BrowseModsPage,
          permission: PERMISSION,
        })
        .addServerRoute({
          name: undefined,
          path: '/vs-mods/mod/:modId',
          element: ModDetailsPage,
          permission: PERMISSION,
        }),
    );
  }
}

export default new CaloptreyxVsModInstallerExtension();

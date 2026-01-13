export type NavigationPosition = 'header' | 'sidebar';

export interface NavigationConfig {
  position: NavigationPosition;
}

export const NAVIGATION_CONFIG: NavigationConfig = {
  position: 'header',
};

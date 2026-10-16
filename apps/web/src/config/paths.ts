/** Single source for route paths so links and redirects cannot drift. */
export const paths = {
  signin: '/signin',
  today: '/today',
  logbook: '/logbook',
  priorities: '/priorities',
  review: (cycleId?: number, section?: string) => `/review${cycleId ? `/${cycleId}${section ? `/${section}` : ''}` : ''}`,
  reviewArchive: (cycleId: number) => `/review/${cycleId}/archive`,
  trends: '/trends',
  settings: (tab?: string) => `/settings${tab ? `/${tab}` : ''}`,
  privacy: '/privacy',
  terms: '/terms',
} as const;

export const settingsTabs = ['profile', 'cycles', 'people', 'integrations', 'ai'] as const;
export type SettingsTab = (typeof settingsTabs)[number];

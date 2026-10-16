import type { LogbookFilter } from './LogbookFilter';
// UI-facing aliases over the generated ts-rs types in this folder (the generated files are the source of truth).
// Only types that exist purely on the client are defined here.
import type { EntryDto } from './EntryDto';

export type { ApiErrorBody } from './client';

export type { UserDto as User } from './UserDto';
export type { CycleDto as Cycle } from './CycleDto';
export type { CycleSummaryDto as CycleSummary } from './CycleSummaryDto';
export type { MeResponse as Me } from './MeResponse';
export type { PublicConfig } from './PublicConfig';
export type { EntryDto as Entry } from './EntryDto';
export type { EntryPage as EntriesPage } from './EntryPage';
export type { Facets as EntryFacets } from './Facets';
export type { EntryTypeDto } from './EntryTypeDto';
export type { EntryInput } from './EntryInput';
export type { BulkRequest } from './BulkRequest';
export type { ClassifyRequest } from './ClassifyRequest';
export type { TodayResponse as Today } from './TodayResponse';
export type { TodayDay } from './TodayDay';
export type { TodayPriority } from './TodayPriority';
export type { MeetingDto } from './MeetingDto';
export type { PrepResponse } from './PrepResponse';
export type { SavePrepRequest } from './SavePrepRequest';
export type { PrioritiesResponse } from './PrioritiesResponse';
export type { PriorityDto as Priority } from './PriorityDto';
export type { PriorityUpdateDto as PriorityUpdate } from './PriorityUpdateDto';
export type { GoalDto as Goal } from './GoalDto';
export type { GoalGroup } from './GoalGroup';
export type { CreatePriorityRequest } from './CreatePriorityRequest';
export type { UpdatePriorityRequest } from './UpdatePriorityRequest';
export type { PostUpdateRequest } from './PostUpdateRequest';
export type { ImportResult } from './ImportResult';
export type { ReviewResponse } from './ReviewResponse';
export type { ReviewSectionDto as ReviewSection } from './ReviewSectionDto';
export type { ReviewSectionConfig } from './ReviewSectionConfig';
export type { ArchiveResponse as Archive } from './ArchiveResponse';
export type { ReportDto as Report } from './ReportDto';
export type { ReportListItem as ReportMeta } from './ReportListItem';
export type { LogbookFilter } from './LogbookFilter';
export type { ReportStats } from './ReportStats';
export type { ReportKeyEntry } from './ReportKeyEntry';
export type { CreateReportRequest as ReportInput } from './CreateReportRequest';
export type { TrendsResponse as Trends } from './TrendsResponse';
export type { TrendRange } from './TrendRange';
export type { NameCount } from './NameCount';
export type { WeekBucket } from './WeekBucket';
export type { ConfigResponse as AppConfig } from './ConfigResponse';
export type { ClgLevelConfig as ClgLevel } from './ClgLevelConfig';
export type { ServiceMeta } from './ServiceMeta';
export type { ConfigField } from './ConfigField';
export type { IntegrationDto as Integration } from './IntegrationDto';
export type { IntegrationInput } from './IntegrationInput';
export type { IntegrationsResponse } from './IntegrationsResponse';
export type { SyncStatus } from './SyncStatus';
export type { TestResult } from './TestResult';
export type { CycleBundle } from './CycleBundle';
export type { UpdateProfileRequest } from './UpdateProfileRequest';
export type { PersonDto as PeopleAlias } from './PersonDto';
export type { UpsertPersonRequest as PeopleAliasInput } from './UpsertPersonRequest';

export type ContentFormat = 'markdown' | 'blocknote';
export type EntrySource = 'github' | 'phabricator' | 'bugzilla' | 'jira' | 'confluence' | 'google_calendar' | 'google_drive' | 'manual';

/** Logbook filters as they appear in the URL, in `GET /entries` and in saved reports. */
export type EntryFilters = LogbookFilter;

/** List rows only need a few fields; the list endpoint returns full DTOs. */
export type EntrySummary = EntryDto;

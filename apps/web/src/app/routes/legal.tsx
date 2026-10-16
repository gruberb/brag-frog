import { Link } from 'react-router';
import { paths } from '@/config/paths';

const Page = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <div className="signin" style={{ alignItems: 'start' }}>
    <div className="card" style={{ maxWidth: 680, padding: 32 }}>
      <h1>{title}</h1>
      <div className="col" style={{ margin: '16px 0', gap: 12 }}>{children}</div>
      <Link className="link" to={paths.signin}>← Back to sign in</Link>
    </div>
  </div>
);

export const Privacy = () => (
  <Page title="Privacy">
    <p>Brag Frog stores the work you log, and what synced services report for you, in your instance's database. Titles, descriptions and notes are encrypted per user.</p>
    <p>AI drafts are only generated when you add your own API key. Nothing is shared with other users.</p>
  </Page>
);
export const Terms = () => (
  <Page title="Terms">
    <p>This instance is provided by your organization for personal work tracking and review preparation. You are responsible for the content you log.</p>
  </Page>
);

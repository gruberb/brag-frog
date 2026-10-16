import { Link } from 'react-router';
import { paths } from '@/config/paths';
import { usePublicConfig } from '../api/auth';

export function SigninPage() {
  const cfg = usePublicConfig();
  return (
    <div className="signin">
      <div className="card box">
        <div className="brand" style={{ justifyContent: 'center', fontSize: 22 }}>
          <img src="/favicon.svg" alt="" /> Brag Frog
        </div>
        <h1>Your work, remembered</h1>
        <p className="muted">Log work from GitHub, Phabricator, Bugzilla, Jira and Calendar. Write better reviews.</p>
        {/* OAuth stays a server redirect, so this must be a real navigation and not a client route. */}
        <a className="btn primary" style={{ width: '100%', padding: '10px' }} href="/auth/login">
          Sign in with Google
        </a>
        <p className="small muted" style={{ marginTop: 12 }}>
          {cfg.data?.instance_name ?? 'Brag Frog'}
          {cfg.data?.allowed_domain_hint ? ` · restricted to ${cfg.data.allowed_domain_hint}` : ''} · <Link to={paths.privacy}>Privacy</Link> ·{' '}
          <Link to={paths.terms}>Terms</Link>
        </p>
      </div>
    </div>
  );
}

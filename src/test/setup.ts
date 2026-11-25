import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

// Times are shown in the computer's time zone; tests read them in UTC wherever they run.
process.env.TZ = 'UTC';

afterEach(() => cleanup());

// Where the mod looks for the CLI the app installs beside it (ADR-047).

import { expect, test } from 'claude-code/testing'
import { installedCli } from '../hooks/register'

test('the installed CLI sits in bin beside the mod folder', () => {
  expect(installedCli('C:\\Users\\u\\AppData\\Local\\TokenRadar\\claude-code')).toBe(
    'C:\\Users\\u\\AppData\\Local\\TokenRadar\\bin\\tokenradar.exe',
  )
  expect(installedCli('C:/apps/TokenRadar/claude-code')).toBe('C:/apps/TokenRadar/bin/tokenradar.exe')
  expect(installedCli('/opt/tokenradar/claude-code')).toBe('/opt/tokenradar/bin/tokenradar')
  expect(installedCli('claude-code')).toBe(null)
})

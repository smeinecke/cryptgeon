import { expect, test } from '@playwright/test'
import { Files, getFileChecksum } from '../../files'
import { checkLinkForDownload } from '../../utils'

test.describe('@web', () => {
  test('retry after failed submit preserves file content', async ({ page }) => {
    const checksum = await getFileChecksum(Files.PDF)

    // Fail the first create request; let subsequent ones reach the server.
    let attempts = 0
    await page.route('**/api/v3/notes', async (route) => {
      if (route.request().method() !== 'POST') return route.continue()
      attempts += 1
      if (attempts === 1) return route.fulfill({ status: 500, body: 'boom' })
      return route.continue()
    })

    await page.goto('/')
    await page.getByTestId('switch-file').click()
    const [fileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.locator('text=No Files Selected').click(),
    ])
    await fileChooser.setFiles([Files.PDF])
    await page.locator('button:has-text("create")').click()
    await expect(page.locator('._toastContainer')).toContainText('could not create note')

    // Retry - the first attempt must not have detached the file buffers.
    await page.locator('button:has-text("create")').click()
    const link = await page.getByTestId('share-link').inputValue()

    await checkLinkForDownload(page, { link, text: 'AES.pdf', checksum })
    expect(attempts).toBe(2)
  })
})

import { expect, test } from '@playwright/test'
import { checkLinkForText, createNoteSuccessfully } from '../../utils'

test.describe('@web', () => {
  test('only http(s) links become clickable', async ({ page }) => {
    const text = [
      'javascript://%0aalert(document.location.hash)',
      'data:text/html,<b>bold</b>',
      'https://example.com is fine',
    ].join('\n')
    const link = await createNoteSuccessfully(page, { text })
    await checkLinkForText(page, { link, text })

    const anchors = page.getByTestId('result').locator('.links a')
    await expect(anchors).toHaveCount(1)
    await expect(anchors.first()).toHaveAttribute('href', 'https://example.com')
    await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0)
    // Note text is rendered escaped - no markup from the payload becomes DOM.
    await expect(page.getByTestId('result').locator('.note').locator('b')).toHaveCount(0)
  })
})

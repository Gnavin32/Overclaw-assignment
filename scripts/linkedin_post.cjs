const { chromium } = require('playwright');

(async () => {
    const postContent = process.argv[2] || "Hello world from Personaliz!";

    console.log(`Posting to LinkedIn: ${postContent}`);

    const browser = await chromium.launch({ headless: false }); // Show browser for demo
    const context = await browser.newContext();
    const page = await context.newPage();

    try {
        await page.goto('https://www.linkedin.com/login');
        // NOTE: For demo purposes, we assume the user is already logged in or will log in manually.
        // In a real scenario, we'd handle session storage or login.

        console.log("Waiting for user to be on feed...");
        await page.waitForURL('**/feed/**', { timeout: 60000 });

        // click 'Start a post'
        await page.click('button.artdeco-button--muted.artdeco-button--4.artdeco-button--tertiary.share-box-feed-entry__trigger');

        // Wait for editor
        await page.waitForSelector('.ql-editor');
        await page.fill('.ql-editor', postContent);

        // Click post
        await page.click('.share-actions__primary-action');

        console.log("Post successful!");
    } catch (e) {
        console.error("Failed to post:", e);
    } finally {
        await new Promise(r => setTimeout(r, 5000)); // Keep open for 5s to see success
        await browser.close();
    }
})();

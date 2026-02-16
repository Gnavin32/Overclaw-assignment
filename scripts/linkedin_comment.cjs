const { chromium } = require('playwright');

(async () => {
    const hashtag = process.argv[2] || "#openclaw";
    const commentText = "Check out our new GitHub repo for OpenClaw! If you're non-technical, try the desktop app for easy automation. 🚀";

    console.log(`Searching for hashtag: ${hashtag}`);

    const browser = await chromium.launch({ headless: false });
    const context = await browser.newContext();
    const page = await context.newPage();

    try {
        await page.goto('https://www.linkedin.com/login');
        console.log("Waiting for user to be on feed...");
        await page.waitForURL('**/feed/**', { timeout: 60000 });

        // Search
        await page.goto(`https://www.linkedin.com/search/results/all/?keywords=${encodeURIComponent(hashtag)}`);
        await page.waitForTimeout(5000);

        // Find comment buttons
        const commentButtons = await page.$$('button.comment-button');
        console.log(`Found ${commentButtons.length} posts to comment on.`);

        if (commentButtons.length > 0) {
            await commentButtons[0].click();
            await page.waitForSelector('.ql-editor');
            await page.fill('.ql-editor', commentText);
            await page.press('.ql-editor', 'Enter');
            console.log("Commented on the first post!");
        }
    } catch (e) {
        console.error("Failed to comment:", e);
    } finally {
        await new Promise(r => setTimeout(r, 5000));
        await browser.close();
    }
})();

// TC-U-001: wkmp-dr Tick-to-Seconds Conversion Test
// Standalone Node.js test script

const TICK_RATE = 28224000;

function ticksToSeconds(ticks) {
    if (ticks === null) return null;
    const seconds = ticks / TICK_RATE;
    return seconds.toFixed(6);
}

function testTicksToSeconds() {
    const testCases = [
        { ticks: 0, expected: "0.000000" },
        { ticks: 28224000, expected: "1.000000" },
        { ticks: 141120000, expected: "5.000000" },
        { ticks: 14112000, expected: "0.500000" },
        { ticks: 1411200, expected: "0.050000" },
        { ticks: 5091609600, expected: "180.400000" },
        { ticks: -28224000, expected: "-1.000000" }
    ];

    let passed = 0;
    let failed = 0;

    console.log("TC-U-001: JavaScript Tick-to-Seconds Conversion Test\n");
    console.log("=" .repeat(60));

    testCases.forEach(tc => {
        const result = ticksToSeconds(tc.ticks);
        if (result === tc.expected) {
            console.log(`✅ PASS: ${tc.ticks.toLocaleString()} ticks → ${result}s`);
            passed++;
        } else {
            console.error(`❌ FAIL: ${tc.ticks.toLocaleString()} ticks → ${result}s (expected ${tc.expected}s)`);
            failed++;
        }
    });

    console.log("=" .repeat(60));
    console.log(`\nResults: ${passed} passed, ${failed} failed`);

    if (failed === 0) {
        console.log("\n✅ TC-U-001: PASS - All conversion tests passed");
        process.exit(0);
    } else {
        console.error("\n❌ TC-U-001: FAIL - Some conversion tests failed");
        process.exit(1);
    }
}

// Run the test
testTicksToSeconds();

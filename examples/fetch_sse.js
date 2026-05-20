// SSE test against echo.websocket.org/.sse
var SSE_URL = "https://echo.websocket.org/.sse";

console.log("Connecting to " + SSE_URL + " ...");
var evtSource = new EventSource(SSE_URL);
console.log("EventSource readyState: " + evtSource.readyState);

if (evtSource.readyState === 1) {
    console.log("Reading SSE events (need at least 3 time events)...");
    var timeCount = 0;
    var maxEvents = 10;

    for (var i = 0; i < maxEvents; i++) {
        var evt = evtSource.read(10000);
        if (evt === undefined || evt === "__CLOSED__") {
            console.log("No more events (or timeout)");
            break;
        }
        console.log("Event: type=" + evt.event + " data=" + evt.data + " id=" + evt.lastEventId);
        if (evt.event === "time") {
            timeCount++;
            if (timeCount >= 3) {
                console.log("SUCCESS: Received 3 time events, exiting");
                break;
            }
        }
    }

    if (timeCount < 3) {
        console.log("PARTIAL: Only got " + timeCount + " time events");
    }

    evtSource.close();
} else {
    console.log("FAILED: EventSource not open");
}

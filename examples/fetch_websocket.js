// WebSocket echo test against wss://echo.websocket.org
// recv(timeout_ms): 0 = pure NIO, >0 = poll with timeout

var WS_URL = "wss://echo.websocket.org";

console.log("Connecting to " + WS_URL + " ...");
var socket = new WebSocket(WS_URL);
console.log("WebSocket readyState: " + socket.readyState + " (1=OPEN)");

var msg = "Hello server!";
console.log("Sending: " + msg);
socket.send(msg);

console.log("Waiting for response (timeout: 10s)...");
var response = socket.recv(10000);

if (response !== undefined) {
    if (response === "__CLOSED__") {
        console.log("Connection closed by server");
    } else {
        console.log("Received: '" + response + "'");
        socket.close(1000, "Done");
        console.log("SUCCESS: WebSocket test passed!");
    }
} else {
    console.log("No response received (timeout)");
    socket.close(1000, "Timeout");
}

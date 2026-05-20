// Download https://example.org and print the HTML
console.log("Fetching https://example.org ...");

fetch("https://example.org")
    .then(function(resp) {
        console.log("Status: " + resp.status);
        console.log("OK: " + resp.ok);
        return resp.text();
    })
    .then(function(body) {
        console.log("Body length: " + body.length);
        console.log("--- Full body text ---");
        console.log(body);
        console.log("--- END ---");
        console.log("SUCCESS: fetch completed");
    })
    .catch(function(err) {
        console.log("ERROR: " + err);
    });

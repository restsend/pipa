fetch("http://example.com/")
    .then(function(r) {
        // Use toString to see the object
        console.log("typeof statusText: " + typeof r.statusText);
        console.log("statusText value: '" + r.statusText + "'");
        console.log("status: " + r.status);
        console.log("ok: " + r.ok);
        return r.text();
    })
    .then(function(t) {
        console.log("text() returned: typeof=" + typeof t);
        console.log("text() length: " + t.length);
        if (typeof t === "string") {
            console.log("First 50: " + t.substring(0, 50));
        }
    })
    .catch(function(e) {
        console.log("ERROR: " + e);
    });

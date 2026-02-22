// clutterlog

document.addEventListener("DOMContentLoaded", function () {
    const grid = document.getElementById("grid");
    if (!grid || typeof CLUTTERLOG_DATA === "undefined") return;

    // Sort by datetime descending (newest first)
    const sorted = CLUTTERLOG_DATA.slice().sort(function (a, b) {
        return new Date(b.datetime) - new Date(a.datetime);
    });

    sorted.forEach(function (entry) {
        var item = document.createElement("div");
        item.className = "grid-item";

        var img = document.createElement("img");
        img.src = entry.image_url;
        img.alt = entry.description;
        img.loading = "lazy";

        var overlay = document.createElement("div");
        overlay.className = "overlay";

        var title = document.createElement("div");
        title.className = "item-title";
        title.textContent = entry.title;

        var description = document.createElement("div");
        description.className = "item-description";
        description.textContent = entry.description;

        var date = document.createElement("div");
        date.className = "item-date";
        date.textContent = formatDate(entry.datetime);

        overlay.appendChild(title);
        overlay.appendChild(description);
        overlay.appendChild(date);

        item.appendChild(img);
        item.appendChild(overlay);
        grid.appendChild(item);
    });
});

function formatDate(datetime) {
    var d = new Date(datetime);
    return d.toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
        day: "numeric",
    });
}

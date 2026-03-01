// clutterlog

(function () {
    "use strict";

    var sorted = [];
    var currentIndex = -1;
    var zoomLevel = 1.0;

    var ZOOM_STEP = 0.25;
    var ZOOM_SCROLL_STEP = 0.1;
    var ZOOM_MIN = 0.5;
    var ZOOM_MAX = 5.0;

    var ANIMATED_EXTENSIONS = ["gif"];
    var VIDEO_EXTENSIONS = ["mp4", "webm"];

    // --- Utility ---

    function formatDate(datetime) {
        var d = new Date(datetime);
        return d.toLocaleDateString("en-US", {
            year: "numeric",
            month: "short",
            day: "numeric",
        });
    }

    function getExtension(url) {
        var parts = url.split(".");
        return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : "";
    }

    function getFilename(url) {
        var parts = url.split("/");
        return parts[parts.length - 1];
    }

    // --- Grid ---

    function buildGrid() {
        var grid = document.getElementById("grid");
        if (!grid || typeof CLUTTERLOG_DATA === "undefined") return;

        sorted = CLUTTERLOG_DATA.slice().sort(function (a, b) {
            return new Date(b.datetime) - new Date(a.datetime);
        });

        sorted.forEach(function (entry, index) {
            var item = document.createElement("div");
            item.className = "grid-item";

            var img = document.createElement("img");
            img.src = entry.thumb_url;
            img.alt = entry.description || entry.title;
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

            item.addEventListener("click", function () {
                openLightbox(index);
            });
        });
    }

    // --- Lightbox ---

    function openLightbox(index) {
        if (index < 0 || index >= sorted.length) return;

        currentIndex = index;
        zoomLevel = 1.0;

        var entry = sorted[index];
        var ext = getExtension(entry.image_url);
        var content = document.getElementById("lightbox-content");
        var info = document.getElementById("lightbox-info");
        var lightbox = document.getElementById("lightbox");

        content.innerHTML = "";
        content.className = "lightbox-content";

        var mediaEl;
        if (VIDEO_EXTENSIONS.indexOf(ext) !== -1) {
            mediaEl = document.createElement("video");
            mediaEl.src = entry.image_url;
            mediaEl.controls = true;
            mediaEl.autoplay = true;
            mediaEl.loop = true;
            mediaEl.playsInline = true;
        } else {
            mediaEl = document.createElement("img");
            mediaEl.src = entry.image_url;
            mediaEl.alt = entry.description || entry.title;
        }

        mediaEl.id = "lightbox-media";
        content.appendChild(mediaEl);

        info.innerHTML = "";
        var titleEl = document.createElement("div");
        titleEl.className = "lightbox-title";
        titleEl.textContent = entry.title;
        info.appendChild(titleEl);

        if (entry.description) {
            var descEl = document.createElement("div");
            descEl.className = "lightbox-description";
            descEl.textContent = entry.description;
            info.appendChild(descEl);
        }

        var dateEl = document.createElement("div");
        dateEl.className = "lightbox-date";
        dateEl.textContent = formatDate(entry.datetime);
        info.appendChild(dateEl);

        lightbox.hidden = false;
        document.body.style.overflow = "hidden";

        applyZoom();

        var filename = getFilename(entry.image_url);
        history.replaceState(null, "", "#media=" + encodeURIComponent(filename));
    }

    function closeLightbox() {
        var lightbox = document.getElementById("lightbox");
        var content = document.getElementById("lightbox-content");

        // Stop any playing video
        var video = content.querySelector("video");
        if (video) {
            video.pause();
            video.src = "";
        }

        lightbox.hidden = true;
        content.innerHTML = "";
        document.body.style.overflow = "";
        currentIndex = -1;
        zoomLevel = 1.0;

        history.replaceState(null, "", window.location.pathname + window.location.search);
    }

    function navigateLightbox(delta) {
        if (currentIndex === -1 || sorted.length === 0) return;
        var newIndex = (currentIndex + delta + sorted.length) % sorted.length;
        openLightbox(newIndex);
    }

    // --- Zoom ---

    function applyZoom() {
        var mediaEl = document.getElementById("lightbox-media");
        var content = document.getElementById("lightbox-content");
        if (!mediaEl) return;

        mediaEl.style.transform = "scale(" + zoomLevel + ")";

        if (zoomLevel > 1.0) {
            content.className = "lightbox-content zoomed";
        } else {
            content.className = "lightbox-content";
        }
    }

    function adjustZoom(delta) {
        zoomLevel = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoomLevel + delta));
        applyZoom();
    }

    function resetZoom() {
        zoomLevel = 1.0;
        applyZoom();
    }

    // --- URL hash deep linking ---

    function openFromHash() {
        var hash = window.location.hash;
        if (!hash || hash.indexOf("#media=") !== 0) return;

        var filename = decodeURIComponent(hash.substring(7));
        if (!filename) return;

        for (var i = 0; i < sorted.length; i++) {
            if (getFilename(sorted[i].image_url) === filename) {
                openLightbox(i);
                return;
            }
        }
    }

    // --- Event binding ---

    function bindEvents() {
        // Close button
        document.getElementById("lightbox-close").addEventListener("click", closeLightbox);

        // Backdrop click
        document.getElementById("lightbox-backdrop").addEventListener("click", closeLightbox);

        // Nav buttons
        document.getElementById("lightbox-prev").addEventListener("click", function (e) {
            e.stopPropagation();
            navigateLightbox(-1);
        });
        document.getElementById("lightbox-next").addEventListener("click", function (e) {
            e.stopPropagation();
            navigateLightbox(1);
        });

        // Zoom buttons
        document.getElementById("lightbox-zoom-in").addEventListener("click", function () {
            adjustZoom(ZOOM_STEP);
        });
        document.getElementById("lightbox-zoom-out").addEventListener("click", function () {
            adjustZoom(-ZOOM_STEP);
        });
        document.getElementById("lightbox-zoom-reset").addEventListener("click", resetZoom);

        // Keyboard
        document.addEventListener("keydown", function (e) {
            if (currentIndex === -1) return;

            switch (e.key) {
                case "Escape":
                    closeLightbox();
                    break;
                case "ArrowLeft":
                    navigateLightbox(-1);
                    break;
                case "ArrowRight":
                    navigateLightbox(1);
                    break;
                case "+":
                case "=":
                    adjustZoom(ZOOM_STEP);
                    break;
                case "-":
                    adjustZoom(-ZOOM_STEP);
                    break;
                case "0":
                    resetZoom();
                    break;
            }
        });

        // Mouse wheel zoom on lightbox content
        document.getElementById("lightbox-content").addEventListener("wheel", function (e) {
            if (currentIndex === -1) return;
            e.preventDefault();
            var delta = e.deltaY < 0 ? ZOOM_SCROLL_STEP : -ZOOM_SCROLL_STEP;
            adjustZoom(delta);
        }, { passive: false });

        // Hash change (browser back/forward)
        window.addEventListener("hashchange", function () {
            var hash = window.location.hash;
            if (!hash || hash.indexOf("#media=") !== 0) {
                if (currentIndex !== -1) {
                    closeLightbox();
                }
            } else {
                openFromHash();
            }
        });
    }

    // --- Init ---

    document.addEventListener("DOMContentLoaded", function () {
        buildGrid();
        bindEvents();
        openFromHash();
    });
})();

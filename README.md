<h1 align="center">clutterlog</h1>
<p align="center">really simple gallery SSG aimed at displaying project WIPs without much context</p>

<p align="center">
  <img src="media/clutterlog-example.gif" alt="clutterlog gallery demo" width="720">
</p>

**clutterlog** is a static gallery website generator for your creative WIPs. Drop photos, GIFs, and videos into a folder and clutterlog builds gallery site that displays them in chronological order. No curation, no context, just a dump of your creative mess. Let people see how you create.

## ✨ Features

- **Multiple media formats**: PNG, JPEG, WebP, GIF, MP4, and WebM
- **Automatic thumbnails**: center-cropped, square thumbnails for every media file
- **Animated thumbnails**: GIFs and videos get 2-second looping animated WebP thumbnails
- **Lazy loading**: thumbnails load on demand for fast initial page loads
- **Deep linking**: each media item is addressable via URL hash (`#media=filename`), supporting direct links and browser back/forward
- **Lightbox viewer**: fullscreen media viewer with zoom controls, keyboard navigation, and previous/next browsing

## 📋 Prerequisites

- **Rust**: 1.90 or later, only if you're installing it with `cargo`
- **ffmpeg**: required for generating animated thumbnails from GIFs, WebM, and MP4 files

## 📥 Install

```shell
cargo install --git https://github.com/CrociDB/clutterlog
```

## 🚀 Usage

### Create a new site

```shell
clutterlog new my_clutterlog
cd my_clutterlog
```

This creates a directory with a default `site.toml` and an empty `media/` folder.

### Add media and build

Edit `site.toml` with your site info, drop your files into `media/`, then:

```shell
clutterlog build
```

This scans your media, extracts metadata, generates thumbnails, and outputs a complete static site to `build/`.

### Update metadata

```shell
clutterlog update
```

Syncs the `.clutterlog/metamedia.toml` file with the current contents of `media/` — adding entries for new files and removing stale ones. See [Media Metadata](#-media-metadata) for why this matters.

## ⚙️ Site Configuration

The `site.toml` file controls your gallery's metadata:

```toml
title = "my_clutterlog"
description = "An uncurated timeline of unfinished projects"
author = "author-name"
url = "https://localhost:8088/"
```

| Field         | Description                                              |
|---------------|----------------------------------------------------------|
| `title`       | Site title, shown in the header and browser tab          |
| `description` | Tagline shown below the title                            |
| `author`      | Your name, shown in the footer                           |
| `url`         | Base URL used for constructing absolute media URLs       |

## 🗃️ Media Metadata

When you add media files to your `media/` folder, clutterlog extracts the best available date for each file — first from EXIF metadata, then falling back to filesystem creation and modification times. These dates are stored in `.clutterlog/metamedia.toml` and used to sort the gallery chronologically.

**⚠️ Why this matters:** Filesystem timestamps (created/modified) are not preserved by git. Every clone, checkout, or pull resets them to the current time, which would destroy your chronological ordering. By persisting dates in `metamedia.toml`, clutterlog ensures your timeline stays correct regardless of git operations.

**Recommended workflow:**

1. Add your media files to `media/`
2. Run `clutterlog update` to extract and store dates for new files
3. Commit both your media files **and** `.clutterlog/metamedia.toml` to git
4. Run `clutterlog build` whenever you want to generate the site

> 💡 The `build` command also runs the metadata update automatically, so step 2 is only necessary if you want to commit the metadata before building.

## 📁 Output Structure

After `clutterlog build`, the `build/` directory contains:

```
build/
  index.html
  public/
    style.css
    clutterlog.js
  media/
    photo.jpg
    photo_thumb.jpg
    clip.mp4
    clip_thumb.webp
    ...
```

This directory is a self-contained static site ready to be deployed to any static hosting service (GitHub Pages, Netlify, Cloudflare Pages, etc). 🚀

## 📄 License

This project is licensed under the [BSD 3-Clause License](LICENSE).

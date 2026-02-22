<h1 align="center">clutterlog</h1>
<p align="center">An uncurated timeline of unfinished projects</p>

---

**clutterlog** is a static gallery website generator for your project's WIPs. Either is that photos, gifs or videos, **clutterlog** will display them in chronological order without much context. It's just a dump of your creative mess. Let people see how you create.

You basically need to create github repo, submit your data and it will generate the website using GitHub actions and publish it using GitHub Pages.

## Install

```shell
cargo install --git https://github.com/CrociDB/clutterlog
```

## Usage

```shell
clutterlog new my_clutterlog
cd my_clutterlog
```

Edit your clutterlog info in `site.toml`, then add all your media in `media/` and build your website:

```shell
clutterlog build
```

That's all you need!


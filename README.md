<p align="center"><strong>Datax CLI</strong> is a coding agent that runs locally on your computer.
<p align="center">
  <img src="https://github.com/mbellary/datax/blob/main/.github/datax-cli-splash.png" alt="Datax CLI splash" width="80%" />
</p>
</p>

---

## Quickstart

### Installing and running Datax CLI

Run the following on Mac or Linux to install Datax CLI:

```shell
curl -fsSL https://github.com/mbellary/datax/releases/latest/download/install.sh | sh
```

Run the following on Windows to install Datax CLI:

```
powershell -ExecutionPolicy ByPass -c "irm https://github.com/mbellary/datax/releases/latest/download/install.ps1 | iex"
```

Datax CLI can also be installed via npm:

```shell
npm install -g datax
```

Then simply run `datax` to get started.

<details>
<summary>You can also go to the <a href="https://github.com/mbellary/datax/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `datax-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `datax-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `datax-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `datax-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `datax-x86_64-unknown-linux-musl`), so you likely want to rename it to `datax` after extracting it.

</details>

### Using Datax

Run `datax` and select the sign-in option appropriate for your environment.

You can also use Datax with an API key.

## Docs

- [**Datax repository**](https://github.com/mbellary/datax)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).

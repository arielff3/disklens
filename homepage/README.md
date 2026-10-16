# Disklens homepage

The landing page for [Disklens](https://github.com/arielff3/disklens), built with Next.js, Tailwind CSS and components from [React Bits](https://reactbits.dev).

```bash
npm install
npm run dev
```

Then open http://localhost:3000.

## Layout

- `app/`: the page, layout, icon and social preview image.
- `components/site/`: the page's sections.
- `components/reactbits/`: React Bits components, copied in from the registry (`https://reactbits.dev/r/<Name>-TS-TW.json`). Their light themes are tuned to the Sand palette.
- `lib/site.ts`: the version and download links. Bump `VERSION` with each release.
- `public/`: screenshots, copied from the repo's `docs/screenshots`.

## Deploy on Vercel

Import the repository and set **Root Directory** to `homepage`. Vercel detects Next.js; no other settings are needed.

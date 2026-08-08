# Localization

netmnt follows the usual Linux/KDE localization mechanisms:

- English strings in the Rust source are gettext message identifiers and the
  canonical fallback text.
- `po/netmnt.pot` is the gettext template. Each translation is a sibling PO
  file named for its locale, currently `po/fr.po`.
- `data/servicemenus/*.desktop` uses Freedesktop/KDE localized keys such as
  `Name[fr]`.
- `data/polkit/org.netmnt.policy` uses polkit's localized `description` and
  `message` elements with `xml:lang`.

The compiled MO files are generated below `build/locale`; they are not tracked.
`make install` compiles and installs them as
`$PREFIX/share/locale/<language>/LC_MESSAGES/netmnt.mo`. `make uninstall`
removes the installed catalogs and `make clean` removes local generated ones.

## Updating messages

GNU gettext development tools (`xgettext`, `msgmerge`, and `msgfmt`) are needed.
After adding or changing a translatable Rust string, run:

```sh
make update-translations
```

This extracts calls to `tr(...)` and `tr_args(...)` into `po/netmnt.pot`, then
merges the template into every PO file listed by `LINGUAS` in the `Makefile`.
Translate any new empty or fuzzy entries, preserving named placeholders such as
`{url}` and `{path}`, and validate the result with:

```sh
make i18n
```

Desktop and polkit translations are maintained directly in their installed
data files because those formats have their own standard localization support.

## Adding a language

For a locale such as `de`:

1. Run `msginit --input=po/netmnt.pot --locale=de --output-file=po/de.po`.
2. Add `de` to `LINGUAS` in the `Makefile`.
3. Translate `po/de.po`, keeping every named placeholder intact.
4. Add matching localized keys to both ServiceMenu files and `xml:lang="de"`
   elements to the polkit policy.
5. Run `make update-translations && make i18n`.

## Locale selection and testing

At startup, each Rust process calls the system `setlocale` and gettext APIs.
GNU gettext therefore uses the standard `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`,
and `LANG` environment settings. If no matching installed catalog exists, or a
message is untranslated, gettext returns the English source text. Catalogs are
decoded as UTF-8.

For an installed build, test CLI text explicitly (the named locales must be
generated on the system):

```sh
LC_ALL=C netmnt --help
LC_ALL=fr_FR.UTF-8 netmnt --help
```

To test the catalogs from the working tree without installing them:

```sh
make i18n
NETMNT_LOCALEDIR="$PWD/build/locale" LC_ALL=C cargo run -p netmnt -- --help
NETMNT_LOCALEDIR="$PWD/build/locale" LC_ALL=fr_FR.UTF-8 cargo run -p netmnt -- --help
```

If the French locale is not generated locally, select French through gettext's
`LANGUAGE` variable while using any available non-`C` UTF-8 locale, for example
`LANGUAGE=fr LC_ALL=en_US.UTF-8`.

Launch Dolphin with the same locale to inspect ServiceMenu labels. Polkit's
authentication agent selects the localized policy message for the desktop
session. The privileged daemon is a system service and uses the system service
locale for its detailed D-Bus error text; the client-facing success, prompt,
warning, and notification text uses the invoking user's locale.

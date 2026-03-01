pkgname=emojiclip
pkgver=0.1.0
pkgrel=1
pkgdesc='Emoji and Unicode symbol picker with fuzzy search and clipboard copy'
arch=('x86_64')
license=('MIT')
depends=('gtk4')
makedepends=('cargo')
options=(!debug)

build() {
  cd "$startdir"
  cargo build --release
}

package() {
  cd "$startdir"
  install -Dm755 target/release/emojiclip "$pkgdir/usr/bin/emojiclip"
  install -Dm644 emojiclip.desktop "$pkgdir/usr/share/applications/emojiclip.desktop"
}

# Maintainer: Decator <decator.c@proton.me>
pkgname="rustic-raven-git"
_pkgname="rustic-raven"
pkgver=0.1.0
pkgrel=1
pkgdesc="A static site generator"
arch=("x86_64" "x86")
url="https://github.com/El-Wumbus/RusticRaven"
license=("APACHE")
provides=("rinfo")
makedepends=("rust")
source=( "$_pkgname::git+https://github.com/El-Wumbus/RusticRaven.git")
sha256sums=("SKIP")

build() {
  cd "$_pkgname"
  cargo build --release
}

package() {
  cd "$_pkgname"
  install target/release/raven -Dm755 "${pkgdir}/usr/bin/raven"
}
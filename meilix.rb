# Homebrew formula. Host in a repo named `homebrew-<tap>`, e.g. johnoppenheimer/homebrew-tap,
# then: brew install johnoppenheimer/tap/meilix
class Meilix < Formula
  desc "Terminal UI to manage Meilisearch indexes"
  homepage "https://github.com/johnoppenheimer/meilix"
  # Point at the crates.io tarball (or a GitHub release tarball).
  url "https://static.crates.io/crates/meilix/meilix-0.1.0.crate"
  sha256 "f028eabfd615601bb2df5e2c8444387405dc705d1f1618ac8bd9f0eb124446db"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "meilix", shell_output("#{bin}/meilix --help")
  end
end

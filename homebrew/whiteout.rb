class Whiteout < Formula
  desc "Git filter tool that prevents secrets from being committed"
  homepage "https://github.com/bytware/whiteout"
  url "https://github.com/bytware/whiteout/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "c09381cdcc877faa74f1d5e20e164cc7c1e74e2beb4faa41200c89b7cf0673cf"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/whiteout", "--version"
  end
end
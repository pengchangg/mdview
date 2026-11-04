# 由 scripts/formula.sh 0.4.0 生成，别手改：sha256 来自已发布的 tarball。
class Mdview < Formula
  desc "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
  homepage "https://github.com/pengchangg/mdview"
  version "0.4.0"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.0/mdview-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e0729afde7dc4de3656d060d714c3a6e7ae0130b0fa37d67842627aa0771780c"
    end
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.0/mdview-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e8b588b0423b06bde94a84e8ea555894e2c092cef49e8c85a52607961fa3bf64"
    end
  end

  on_macos do
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.0/mdview-aarch64-apple-darwin.tar.gz"
      sha256 "4815a54175b1583addd06bdfc5b821381bc84cf35c9c55ef8f115a701b89ea1b"
    end
  end

  def install
    bin.install "mdview"
  end
end

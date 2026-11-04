# 由 scripts/formula.sh 0.4.1 生成，别手改：sha256 来自已发布的 tarball。
class Mdview < Formula
  desc "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
  homepage "https://github.com/pengchangg/mdview"
  version "0.4.1"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.1/mdview-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "1c86b4010e99034a1e41d758fb6e75fb16c808d833c3d9f3026ac76425188383"
    end
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.1/mdview-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "5de23e34b9342f908443609fbf9555c00a4e8b72e87bfd9541cf1eeece646e4c"
    end
  end

  on_macos do
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.1/mdview-aarch64-apple-darwin.tar.gz"
      sha256 "450535e196e11367e83b97856c6ba68055b3ce354d281848cab405cc68d2ef1f"
    end
  end

  def install
    bin.install "mdview"
  end
end

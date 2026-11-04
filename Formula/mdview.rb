# 由 scripts/formula.sh 0.3.1 生成，别手改：sha256 来自已发布的 tarball。
class Mdview < Formula
  desc "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
  homepage "https://github.com/pengchangg/mdview"
  version "0.3.1"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/pengchangg/mdview/releases/download/v0.3.1/mdview-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "4c2c47045139b09a0a0f6d221a4458a2112e6cf9c5bd0af83265edbb0fd560f4"
    end
  end

  on_macos do
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.3.1/mdview-aarch64-apple-darwin.tar.gz"
      sha256 "8b52f02e3875611d9673580c92d05d445337aa30a15e3c0e065b7299b5b727ea"
    end
  end

  def install
    bin.install "mdview"
  end
end

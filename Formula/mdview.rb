# 由 scripts/formula.sh 0.4.2 生成（发布后由 CI 的 tap job 跑），别手改：sha256 来自已发布的 tarball。
class Mdview < Formula
  desc "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"
  homepage "https://github.com/pengchangg/mdview"
  version "0.4.2"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.2/mdview-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "f044f1e754ec36e2731285b15e75430ec8ffd1e775ec1a24c8caa6c0ce1814b2"
    end
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.2/mdview-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "748128515dbc2381cbfb046a7010a21040ae1ebcaa7a5ecfb2e6011dad87abef"
    end
  end

  on_macos do
    on_arm do
      url "https://github.com/pengchangg/mdview/releases/download/v0.4.2/mdview-aarch64-apple-darwin.tar.gz"
      sha256 "a41f56d0b839c4e21711b54e433cc833265d0b86c55d42b11568a67cda2b1214"
    end
  end

  def install
    bin.install "mdview"
  end
end

class Skillsdash < Formula
  desc "Cross-platform TUI for managing AI skills across Claude and Agents providers"
  homepage "https://github.com/saeedvaziry/skillsdash"
  version "0.1.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "414de191cd948f4a1fc8f5f5d3503961ace60aea332ee662e870819ab2b30175"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "1280e8ff906dd66a9ff744d314a807f3b448e52ea91ff152b61cf4e23693345b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "87affd3a81c5e3f18da4fddd99c1df78d8d4c10900b2aaa6c5111641bc0deaef"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "5df323b3de5a2d9647df4d66d1fbe91a61801f9a4627282f45821f129d3f80cb"
    end
  end

  def install
    bin.install "skillsdash"
  end

  test do
    assert_match "skillsdash", shell_output("#{bin}/skillsdash --version", 2)
  end
end

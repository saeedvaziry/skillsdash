class Skillsdash < Formula
  desc "Cross-platform TUI for managing AI skills across Claude and Agents providers"
  homepage "https://github.com/saeedvaziry/skillsdash"
  version "0.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "skillsdash"
  end

  test do
    assert_match "skillsdash", shell_output("#{bin}/skillsdash --version", 2)
  end
end

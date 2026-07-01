class Skillsdash < Formula
  desc "Cross-platform TUI for managing AI skills across Claude and Agents providers"
  homepage "https://github.com/saeedvaziry/skillsdash"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "2a429d1c7a4f7052d5da7e07797b1ab99149e6d5f16e4ce3bc77809343bc958a"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0c1105e2eb68f50c3578dcaae601c48c1cbc640f326c3d5b2c9d36c5fd9d1e4a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "593fd807eb182f164a961474aae40890ae506d6515be0f51c0cb4255e6296985"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "384c4cca8d410b036f36cd8ddd22194b8df3267864bef56e2b2910cb4193d81c"
    end
  end

  def install
    bin.install "skillsdash"
  end

  test do
    assert_match "skillsdash", shell_output("#{bin}/skillsdash --version", 2)
  end
end

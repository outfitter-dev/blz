class Blz < Formula
  desc "Fast local search for llms.txt"
  homepage "https://blz.run"
  version "1.1.0"
  license "Apache-2.0"

  livecheck do
    url :stable
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/outfitter-dev/blz/releases/download/v#{version}/blz-#{version}-darwin-arm64.tar.gz"
      sha256 "05a6b6225928bceeeed829e88d44580c4e32648be3fa3a058414532a03cd603d"
    end
    on_intel do
      url "https://github.com/outfitter-dev/blz/releases/download/v#{version}/blz-#{version}-darwin-x64.tar.gz"
      sha256 "f9812332c6ccc546fa3cde7eebf34d496b790051789c859d28abce8a820ce9ce"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/outfitter-dev/blz/releases/download/v#{version}/blz-#{version}-linux-x64.tar.gz"
      sha256 "2b86ac699b301247f042d32f364f3b07cf00b1d7bbb845f65906eea0598135a3"
    end
    on_intel do
      url "https://github.com/outfitter-dev/blz/releases/download/v#{version}/blz-#{version}-linux-x64.tar.gz"
      sha256 "2b86ac699b301247f042d32f364f3b07cf00b1d7bbb845f65906eea0598135a3"
    end
  end

  def install
    if OS.linux? && Hardware::CPU.arm?
      odie "The blz Linux arm64 binary is not available yet. Please use the x86_64 build."
    end
    bin.install "blz"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/blz --version")
    assert_match "blz", shell_output("#{bin}/blz --help")
  end
end

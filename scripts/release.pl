#!/usr/bin/env perl

use strict;
use warnings;

use Cwd qw(abs_path);
use Digest::SHA ();
use File::Basename qw(basename dirname);
use File::Copy qw(copy);
use File::Find qw(find);
use File::Path qw(make_path);
use File::Spec;
use File::Temp qw(tempdir);
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);

my $root = abs_path(File::Spec->catdir($Bin, '..'));
my $sdk_version;
my $target;
my $arch;

GetOptions(
    'sdk-version=s' => \$sdk_version,
    'target=s'      => \$target,
    'arch=s'        => \$arch,
) or die usage();

@ARGV == 0 or die usage();
defined $sdk_version or die usage();
validate_fragment('SDK version', $sdk_version);
$arch //= architecture_name($target);
validate_fragment('architecture', $arch);

my @build = ('cargo', 'build', '--release', '--locked');
push @build, '--target', $target if defined $target;
run(@build);

my $binary_directory = defined $target
    ? File::Spec->catdir($root, 'target', $target, 'release')
    : File::Spec->catdir($root, 'target', 'release');
my $shared_library = File::Spec->catfile($binary_directory, 'libviewkit.so');
require_file($shared_library);

my $stage = tempdir('viewkit-release-XXXXXX', TMPDIR => 1, CLEANUP => 1);
stage_tree(File::Spec->catdir($root, 'lib'), File::Spec->catdir($stage, 'lib'));
stage_file(File::Spec->catfile($root, 'license'), File::Spec->catfile($stage, 'LICENSE'));
stage_file(File::Spec->catfile($root, 'readme.md'), File::Spec->catfile($stage, 'README.md'));
stage_file($shared_library, File::Spec->catfile($stage, 'libviewkit.so'));

my $output_directory = File::Spec->catdir($root, 'target', 'release');
make_path($output_directory);
my $filename = "$arch-viewkit-$sdk_version.tar.zst";
my $archive = File::Spec->catfile($output_directory, $filename);
my $tar = File::Spec->catfile($stage, "$arch-viewkit-$sdk_version.tar");
run(
    'tar', '--sort=name', '--mtime=@' . source_date_epoch(),
    '--owner=0', '--group=0', '--numeric-owner',
    '-C', $stage, '-cf', $tar,
    qw(LICENSE README.md lib libviewkit.so),
);
run('zstd', '-q', '-19', '-f', $tar, '-o', $archive);
write_checksum($archive, File::Spec->catfile($output_directory, 'SHA256SUMS'));
print "$archive\n";

sub usage {
    return "usage: scripts/release.pl --sdk-version <version> [--target <rust-target>] [--arch <artifact-arch>]\n";
}

sub architecture_name {
    my ($target) = @_;
    return (split /-/, $target, 2)[0] if defined $target;
    open my $rustc, '-|', 'rustc', '-vV' or die "failed to start rustc: $!\n";
    my $host;
    while (my $line = <$rustc>) {
        $host = $1 if $line =~ /^host:\s+([^\s]+)/;
    }
    close $rustc or die "rustc -vV failed\n";
    defined $host or die "rustc did not report a host architecture\n";
    return (split /-/, $host, 2)[0];
}

sub validate_fragment {
    my ($description, $value) = @_;
    $value =~ /\A[A-Za-z0-9][A-Za-z0-9._+-]*\z/
        or die "invalid $description: $value\n";
}

sub stage_tree {
    my ($source, $destination) = @_;
    find(
        {
            no_chdir => 1,
            wanted => sub {
                return unless -f $_;
                my $relative = File::Spec->abs2rel($_, $source);
                stage_file($_, File::Spec->catfile($destination, $relative));
            },
        },
        $source,
    );
}

sub stage_file {
    my ($source, $destination) = @_;
    require_file($source);
    make_path(dirname($destination));
    copy($source, $destination) or die "failed to copy $source: $!\n";
}

sub require_file {
    my ($path) = @_;
    -f $path or die "required release file was not found: $path\n";
}

sub source_date_epoch {
    open my $git, '-|', 'git', '-C', $root, 'log', '-1', '--format=%ct'
        or die "failed to start git: $!\n";
    my $epoch = <$git>;
    close $git or die "git log failed\n";
    chomp $epoch;
    $epoch =~ /\A\d+\z/ or die "git returned an invalid source timestamp\n";
    return $epoch;
}

sub write_checksum {
    my ($archive, $path) = @_;
    open my $input, '<', $archive or die "failed to read $archive: $!\n";
    binmode $input;
    my $digest = Digest::SHA->new(256)->addfile($input)->hexdigest;
    close $input or die "failed to close $archive: $!\n";
    open my $output, '>', $path or die "failed to write $path: $!\n";
    print {$output} "$digest  " . basename($archive) . "\n";
    close $output or die "failed to close $path: $!\n";
}

sub run {
    my (@command) = @_;
    print '+ ' . join(' ', @command) . "\n";
    system @command;
    my $status = $? >> 8;
    $status == 0 or die "$command[0] exited with status $status\n";
}

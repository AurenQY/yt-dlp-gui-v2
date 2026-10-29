# Copyright (c) 2026 yt-dlp-gui contributors
# SPDX-License-Identifier: GPL-2.0-only
"""Bounded gallery-dl playlist artifact completion adapter (ADR-0002)."""
import copy
import errno
import hashlib
import os
import shutil
import stat
import uuid
from dataclasses import dataclass
from pathlib import Path


def checked(path, root=None):
    def absolute(value):
        value = os.path.abspath(value)
        # Python/FFmpeg builds cannot assume the host enables long-path policy.
        # Merger filenames add format suffixes beyond the planned media name.
        if os.name == "nt" and not value.startswith("\\\\?\\"):
            value = "\\\\?\\UNC\\" + value[2:] if value.startswith("\\\\") else "\\\\?\\" + value
        return Path(value)

    path = absolute(path)
    if root is not None and not path.is_relative_to(absolute(root)):
        raise ValueError("Gallery artifact escaped its owned directory")
    for parent in (path, *path.parents):
        try:
            info = parent.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"Gallery storage links are not supported: {parent}")
    return path


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def publish(source, destination):
    """Publish without replacing an established destination, including EXDEV."""
    source, destination = checked(source), checked(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)

    def move_new(src, dst):
        if os.name == "nt":
            os.rename(src, dst)  # Windows rename refuses an existing destination.
        else:
            os.link(src, dst)
            os.unlink(src)

    try:
        move_new(source, destination)
        return
    except OSError as error:
        if error.errno != errno.EXDEV and getattr(error, "winerror", None) != 17:
            raise
    temporary = destination.with_name(f".yt-dlp-gui-publish-{uuid.uuid4().hex}.tmp")
    try:
        with open(source, "rb") as incoming, open(temporary, "xb") as outgoing:
            shutil.copyfileobj(incoming, outgoing)
            outgoing.flush()
            os.fsync(outgoing.fileno())
        shutil.copystat(source, temporary)
        if digest(source) != digest(temporary):
            raise OSError("Gallery publication copy failed validation")
        checked(destination)
        move_new(temporary, destination)
        source.unlink()
    finally:
        temporary.unlink(missing_ok=True)


@dataclass
class Artifact:
    metadata: dict
    leaf: dict
    index: int
    final: Path
    work: Path
    template: str
    outcome: str = "planned"
    actual: Path | None = None


def leaves(info):
    for entry in info.get("entries", ()):
        if not entry:
            continue
        if "entries" in entry:
            yield from leaves(entry)
        else:
            yield entry


def final_extension(ytdl, entry):
    extension = ytdl.params.get("final_ext") or entry["ext"]
    from yt_dlp.postprocessor.ffmpeg import resolve_mapping
    processors = (*entry.get("__postprocessors", ()),
                  *ytdl._pps.get("post_process", ()))
    for processor in processors:
        mapping = getattr(processor, "mapping", None)
        if mapping:
            extension = resolve_mapping(extension, mapping)[0] or extension
    return extension


def download_playlist(downloader, ytdl, pathfmt, info):
    job = downloader._gui_job
    job._gui_playlist_handled = True
    original = copy.copy(pathfmt)
    source_metadata = dict(pathfmt.kwdict)
    source_key = str(source_metadata.get("_archive_key", ""))
    succeeded = True
    try:
        if not downloader.part or not isinstance(downloader.partdir, str):
            raise ValueError("A concrete owned playlist part directory is required")
        root = checked(downloader.partdir)
        source_metadata["extension"] = pathfmt.prefix
        prefix = pathfmt.build_filename(source_metadata)
        source_metadata["extension"] = original.kwdict["extension"]
        template = prefix.replace("%", "%%") + "%(playlist_index)s.%(ext)s"
        plans = []
        destinations = set()
        for position, raw in enumerate(leaves(info), 1):
            entry = dict(raw)
            entry.setdefault("playlist_index", position)
            if not entry.get("ext"):
                raise ValueError("Playlist entry has no resolved media extension")
            final_info = dict(entry, ext=final_extension(ytdl, entry))
            downloader._set_outtmpl(ytdl, os.path.join(original.realdirectory, template))
            final = checked(ytdl.prepare_filename(final_info), original.realdirectory)
            identity = os.path.normcase(str(final)).casefold()
            if identity in destinations:
                raise ValueError(f"Ambiguous playlist output collision: {final}")
            destinations.add(identity)
            work = checked(root / f"entry-{position}", root)
            work_template = str(work / template)
            plans.append(Artifact(dict(source_metadata, _archive_key=source_key), entry,
                                  position, final, work, work_template))
        if not plans:
            raise ValueError("Playlist contains no selected media artifacts")
        for plan in plans:
            artifact = copy.copy(original)
            artifact.kwdict = plan.metadata
            artifact.kwdict["extension"] = plan.final.suffix.lstrip(".")
            artifact.extension = artifact.kwdict["extension"]
            artifact.filename = plan.final.name
            artifact.realpath = str(plan.final)
            artifact.path = os.path.join(original.directory, plan.final.name)
            artifact.temppath = str(plan.final)
            job.pathfmt = artifact
            if artifact.exists():
                plan.outcome = "skipped"
                job.handle_skip()
                continue
            checked(plan.work, root).mkdir(parents=True, exist_ok=True)
            downloader._set_outtmpl(ytdl, plan.template)
            downloader.error = None
            try:
                if downloader.rate_dyn is not None:
                    ytdl.params["ratelimit"] = downloader.rate_dyn()
                ytdl.process_info(plan.leaf)
                if downloader.error is not None:
                    raise OSError(str(downloader.error))
                actual = checked(plan.leaf.get("filepath") or plan.leaf["_filename"], plan.work)
                if not actual.is_file():
                    raise OSError("Playlist entry did not produce a complete media file")
                # Postprocessors supply the actual path; do not guess the final extension.
                final = checked(plan.final.parent / actual.name, original.realdirectory)
                if final != plan.final:
                    raise ValueError("Playlist postprocessing output differs from its resolved plan")
                artifact.temppath = str(actual)
                for callback in job.hooks.get("file", ()):
                    callback(artifact)
                from .job import FLAGS
                if FLAGS.DOWNLOAD is not None:
                    raise OSError("Playlist download was interrupted")
                publish(artifact.temppath, final)
                artifact.temppath = artifact.realpath
                artifact.set_mtime()
                plan.actual, plan.outcome = final, "downloaded"
                job.out.success(artifact.path)
                job._skipcnt = 0
                for callback in job.hooks.get("after", ()):
                    callback(artifact)
            except Exception as error:
                succeeded = False
                plan.outcome = "failed"
                job.status |= 4
                job._gui_group_failed = True
                job.log.error("Playlist artifact %s failed: %s", plan.index, error)
                for callback in job.hooks.get("error", ()):
                    callback(artifact)
        if succeeded and job.archive is not None:
            job.archive.add(original.kwdict)
    except Exception as error:
        succeeded = False
        job.status |= 4
        job._gui_group_failed = True
        job.log.error("Playlist artifact planning failed: %s", error)
    finally:
        job.pathfmt = pathfmt
        # Errors belong to the per-artifact/group status. Prevent the upstream
        # downloader loop from retrying an already partially finalized group.
        downloader.error = None
    # The job hook consumes the group, including its failure status, without
    # retrying successful siblings or emitting an aggregate file completion.
    return True

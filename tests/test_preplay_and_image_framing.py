"""Guards for the pre-play prompt, movable static crop and low-priority jobs.

The FFmpeg tests use a generated PPM pattern, no downloaded/copyrighted media.
"""
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VIDEO = (ROOT / 'crates/p3w-platform/src/linux/video_optimizer.rs').read_text()
PLATFORM = (ROOT / 'crates/p3w-platform/src/linux/universal_wallpaper.rs').read_text()
CTRL = (ROOT / 'app/src/cxxqt_object.rs').read_text()
IMAGE = (ROOT / 'app/src/image_optimizer.rs').read_text()
MAIN = (ROOT / 'app/qml/Main.qml').read_text()
IMG_UI = (ROOT / 'app/qml/components/ImageFramingDialog.qml').read_text()
VID_UI = (ROOT / 'app/qml/components/VideoPreparationDialog.qml').read_text()
STORE = (ROOT / 'app/src/settings_store.rs').read_text()
RESTORE = (ROOT / 'app/src/wallpaper_restore.rs').read_text()
BUILD = (ROOT / 'app/build.rs').read_text()

class PreplayAndStaticFraming(unittest.TestCase):
    def test_video_popup_before_play(self):
        self.assertIn('if (info.ok && Boolean(wallpaper.is_video) && info.oversized)', MAIN)
        self.assertIn('videoPreparationDialog.openFor(wallpaper, monitorName, info)', MAIN)
        self.assertIn('prepare_video_before_apply', CTRL)
        self.assertIn('video_preparation_status', CTRL)
        self.assertIn('cancel_video_preparation', CTRL)
        self.assertIn('apply_video_choice', CTRL)
        self.assertIn('apply_original_video', PLATFORM)
        self.assertIn('apply_prepared_video', PLATFORM)

    def test_preparation_does_not_play_original(self):
        prepare_section = VIDEO.split('pub fn request_preparation(')[1].split('pub fn cancel_preparation(')[0]
        self.assertNotIn('apply_video_direct', prepare_section)
        self.assertNotIn('activate_optimized', prepare_section)
        self.assertIn('s.waiting.insert(cache.clone())', prepare_section)
        self.assertIn('s.queued.insert(cache.clone())', prepare_section)
        self.assertIn('s.waiting.contains(path)', VIDEO)
        self.assertTrue((ROOT / 'docs/PREPLAY_AND_STATIC_FRAMING.md').is_file())

    def test_video_modal_cancellation(self):
        for token in ('onClosed:', 'cancel_video_preparation(', 'useOptimized(', 'useOriginal(', 'Timer {'):
            self.assertIn(token, VID_UI)
        self.assertIn('PR_SET_PDEATHSIG', VIDEO)
        self.assertIn('cancel_all_preparations', VIDEO)

    def test_static_framing_and_restore(self):
        for mode in ('"fit"', '"stretch"', '"crop"'):
            self.assertIn(mode, IMAGE)
        self.assertIn('focusX', IMG_UI)
        self.assertIn('focusY', IMG_UI)
        self.assertIn('cropFracW', IMG_UI)
        self.assertIn('zoom', IMG_UI)
        self.assertIn('MouseArea {', IMG_UI)
        self.assertIn('image_optimizer::prepare', CTRL)
        self.assertIn('image_optimizer::prepared', CTRL)
        self.assertIn('remember_wallpaper_with_layout', STORE)
        self.assertIn('remember_video_playback_choice', STORE)
        self.assertIn('video_choice', RESTORE)
        self.assertIn('saved_wallpaper_entries_for', RESTORE)
        self.assertIn('prepare_blocking', RESTORE)
        self.assertIn('protect_active', RESTORE)
        self.assertIn('framed-images', IMAGE)
        self.assertIn('prune_cache', IMAGE)
        self.assertIn('PR_SET_PDEATHSIG', IMAGE)

    def test_qml_files_registered(self):
        self.assertIn('.qml_file("qml/components/VideoPreparationDialog.qml")', BUILD)
        self.assertIn('.qml_file("qml/components/ImageFramingDialog.qml")', BUILD)
        self.assertIn('ImageFramingDialog {', MAIN)
        self.assertIn('VideoPreparationDialog {', MAIN)

    @unittest.skipUnless(shutil.which('ffmpeg') and shutil.which('ffprobe'), 'FFmpeg is not installed')
    def test_ffmpeg_fit_stretch_crop_and_crop_position(self):
        # A 600x200 image with distinct red/blue halves. 100x100 fill must
        # crop different visible regions, not merely squeeze the source.
        with tempfile.TemporaryDirectory() as folder:
            folder = Path(folder)
            source = folder / 'gradient.ppm'
            with source.open('wb') as stream:
                stream.write(b'P6\n600 200\n255\n')
                stream.write((b'\xff\x15\x15' * 300 + b'\x15\x15\xff' * 300) * 200)
            modes = {
                'fit': 'scale=100:100:force_original_aspect_ratio=decrease:flags=lanczos,pad=100:100:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1',
                'stretch': 'scale=100:100:flags=lanczos,setsar=1',
                'crop_left': 'scale=100:100:force_original_aspect_ratio=increase:flags=lanczos,crop=100:100:(iw-100)*0.000:(ih-100)*0.500,setsar=1',
                'crop_right': 'scale=100:100:force_original_aspect_ratio=increase:flags=lanczos,crop=100:100:(iw-100)*1.000:(ih-100)*0.500,setsar=1',
                'crop_zoom': 'scale=100:100:force_original_aspect_ratio=increase:flags=lanczos,scale=trunc(iw*2.000/2)*2:trunc(ih*2.000/2)*2:flags=lanczos,crop=100:100:(iw-100)*0.500:(ih-100)*0.500,setsar=1'
            }
            for name, vf in modes.items():
                output = folder / (name + '.jpg')
                subprocess.run(['ffmpeg', '-nostdin', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(source),
                    '-vf', vf, '-frames:v', '1', '-q:v', '3', '-update', '1', str(output)], check=True, timeout=20)
                p = subprocess.check_output(['ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries',
                    'stream=width,height', '-of', 'csv=p=0', str(output)], text=True, timeout=10).strip()
                self.assertEqual(p, '100,100', name)
            def color(name):
                image = folder / (name + '.jpg')
                buf = subprocess.check_output(['ffmpeg', '-v', 'error', '-i', str(image),
                    '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-frames:v', '1', '-'], timeout=15)
                i = (50 * 100 + 50) * 3
                return buf[i:i+3]
            left, right = color('crop_left'), color('crop_right')
            self.assertGreater(left[0], 170)
            self.assertLess(left[2], 100)
            self.assertGreater(right[2], 170)
            self.assertLess(right[0], 100)

if __name__ == '__main__':
    unittest.main()

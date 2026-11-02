"""Native mod architecture: refuse retired backend builds and keep exact host records."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from scripts import package,setup
class BuildOptions(unittest.TestCase):
    def test_retired_switch_is_rejected_before_build_actions(self):
        with patch.object(sys,"argv",["setup.py","--with-mods"]),patch.object(setup,"RUNNERS",{}):
            with self.assertRaises(SystemExit) as e:setup.main()
            self.assertEqual(e.exception.code,2)
    def test_exact_record_and_legacy_rejection(self):
        with tempfile.TemporaryDirectory() as d:
            exe=Path(d)/"game.exe";exe.write_bytes(b"host")
            stamp=dict(schema_version=2,resource_mods=False,skyline_loader=True,latency_slider=False,exe_sha256=package.sha256(exe))
            path=exe.with_suffix(".build.json");path.write_text(json.dumps(stamp))
            self.assertEqual(package.host_features(str(exe)),dict(resource_mods=False,latency_slider=False,skyline_loader=True))
            stamp["resource_mods"]=True;path.write_text(json.dumps(stamp))
            with self.assertRaisesRegex(SystemExit,"Legacy"):package.host_features(str(exe))
            stamp["resource_mods"]=False;path.write_text(json.dumps(stamp));exe.write_bytes(b"replaced")
            with self.assertRaises(SystemExit):package.host_features(str(exe))
    def test_legacy_backend_cannot_be_packaged_even_with_stale_override(self):
        with self.assertRaisesRegex(SystemExit,"retired"):
            package.check_build_is_current("unused","unused",True,resource_mods=True)
if __name__=="__main__":unittest.main()

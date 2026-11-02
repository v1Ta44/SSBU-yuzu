"""Portable Skyline packaging must ship the exact original NRO and a matching registry."""
from pathlib import Path
import json
import struct
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from scripts import package, release_update

class SkylinePackage(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.work=self.root/'work';self.sd=self.root/'sd'
        self.bin=self.work/'plugin_bin';self.bin.mkdir(parents=True)
        self.nro=self.root/package.ARC_PATH;self.nro.parent.mkdir(parents=True)
        data=bytearray(512);struct.pack_into('<I',data,4,0xc0);data[0x10:0x14]=b'NRO0'
        struct.pack_into('<I',data,0x18,512);struct.pack_into('<I',data,0x28,0x80)
        struct.pack_into('<III',data,0x30,256,256,4096);data[0x40:0x54]=bytes(range(20))
        struct.pack_into('<Ii',data,0x80,0,10);data[0x88:0x92]=b'arcropolis'
        data[0xc0:0xc4]=b'MOD0';struct.pack_into('<i',data,0xcc,0x1140)
        self.nro.write_bytes(data)
        (self.bin/'plugins.txt').write_text('sky_arcropolis 0x7280200000 0x1200 0x200 '+bytes(range(20)).hex()+' D:\\private\\original.nro\n')
        (self.bin/'plugin_map.json').write_text(json.dumps({'sky_arcropolis':{'base':'0x7280200000','image_size':4608,'slot':4194304,'source':'D:\\private\\original.nro'}}))
        pe=bytearray(4096);struct.pack_into('<I',pe,0x3c,0x80);pe[0x80:0x84]=b'PE\0\0';struct.pack_into('<H',pe,0x98,0x20b)
        (self.bin/'sky_arcropolis.dll').write_bytes(pe)
    def test_portable_exact_original_and_registry(self):
        before=self.nro.read_bytes(); files=dict((rel,Path(src)) for src,rel in package.skyline_files(self.work,self.sd,self.root/'out',True))
        self.assertEqual(files[package.ARC_PATH].read_bytes(),before)
        self.assertEqual(len(files),4)
        registry=files['game/plugin_bin/plugins.txt'].read_text()
        self.assertNotIn('private',registry);self.assertEqual(len(registry.splitlines()[1].split()),5)
        meta=json.loads(files['game/plugin_bin/arcropolis.json'].read_text())
        self.assertEqual(meta['nro_sha256'],package.sha256(self.nro))
        self.assertTrue(all(release_update.managed(rel) for rel in files))
    def test_rejects_nro_registry_drift(self):
        data=bytearray(self.nro.read_bytes());data[0x40]=99;self.nro.write_bytes(data)
        with self.assertRaises(SystemExit): package.skyline_files(self.work,self.sd,self.root/'out',True)
    def test_does_not_ship_arbitrary_mods_or_plugins(self):
        for rel in ['sd/ultimate/mods/a/config.json','sd/ultimate/mods/a/plugin.nro','sd/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/other.nro','game/plugin_bin/other.dll','game/plugin_bin/plugin_map.json']:
            self.assertFalse(release_update.managed(rel),rel)
    def test_enabled_loader_requires_original(self):
        self.assertEqual(package.skyline_files(self.work,None,self.root/'out',False),[])
        with self.assertRaises(SystemExit):package.skyline_files(self.work,None,self.root/'out',True)

if __name__=='__main__':unittest.main()

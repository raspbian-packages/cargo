// Copyright © 2017 winapi-rs developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// All files in the project carrying such notice may not be copied, modified, or distributed
// except according to those terms.
use ctypes::{c_uchar, c_ulong};
use shared::guiddef::{GUID, REFGUID};
use shared::minwindef::{BOOL, BYTE, DWORD, UINT, ULONG};
use shared::wtypes::{BSTR, CLIPFORMAT};
use um::oaidl::LPSAFEARRAY;
use um::objidl::{IPersistStream, IPersistStreamVtbl};
use um::objidlbase::{IEnumUnknown, IStream};
use um::ocidl::{IPropertyBag2, PROPBAG2};
use um::propidl::PROPVARIANT;
use um::unknwnbase::{IUnknown, IUnknownVtbl};
use um::wincodec::{
    IWICComponentInfo, IWICComponentInfoVtbl, IWICEnumMetadataItem, IWICImagingFactory,
    IWICImagingFactoryVtbl, IWICMetadataQueryReader, IWICMetadataQueryWriter,
};
use um::winnt::{HRESULT, ULARGE_INTEGER, WCHAR};
DEFINE_GUID!{GUID_MetadataFormatUnknown,
    0xa45e592f, 0x9078, 0x4a7c, 0xad, 0xb5, 0x4e, 0xdc, 0x4f, 0xd6, 0x1b, 0x1f}
DEFINE_GUID!{GUID_MetadataFormatIfd,
    0x537396c6, 0x2d8a, 0x4bb6, 0x9b, 0xf8, 0x2f, 0x0a, 0x8e, 0x2a, 0x3a, 0xdf}
DEFINE_GUID!{GUID_MetadataFormatSubIfd,
    0x58a2e128, 0x2db9, 0x4e57, 0xbb, 0x14, 0x51, 0x77, 0x89, 0x1e, 0xd3, 0x31}
DEFINE_GUID!{GUID_MetadataFormatExif,
    0x1c3c4f9d, 0xb84a, 0x467d, 0x94, 0x93, 0x36, 0xcf, 0xbd, 0x59, 0xea, 0x57}
DEFINE_GUID!{GUID_MetadataFormatGps,
    0x7134ab8a, 0x9351, 0x44ad, 0xaf, 0x62, 0x44, 0x8d, 0xb6, 0xb5, 0x02, 0xec}
DEFINE_GUID!{GUID_MetadataFormatInterop,
    0xed686f8e, 0x681f, 0x4c8b, 0xbd, 0x41, 0xa8, 0xad, 0xdb, 0xf6, 0xb3, 0xfc}
DEFINE_GUID!{GUID_MetadataFormatApp0,
    0x79007028, 0x268d, 0x45d6, 0xa3, 0xc2, 0x35, 0x4e, 0x6a, 0x50, 0x4b, 0xc9}
DEFINE_GUID!{GUID_MetadataFormatApp1,
    0x8fd3dfc3, 0xf951, 0x492b, 0x81, 0x7f, 0x69, 0xc2, 0xe6, 0xd9, 0xa5, 0xb0}
DEFINE_GUID!{GUID_MetadataFormatApp13,
    0x326556a2, 0xf502, 0x4354, 0x9c, 0xc0, 0x8e, 0x3f, 0x48, 0xea, 0xf6, 0xb5}
DEFINE_GUID!{GUID_MetadataFormatIPTC,
    0x4fab0914, 0xe129, 0x4087, 0xa1, 0xd1, 0xbc, 0x81, 0x2d, 0x45, 0xa7, 0xb5}
DEFINE_GUID!{GUID_MetadataFormatIRB,
    0x16100d66, 0x8570, 0x4bb9, 0xb9, 0x2d, 0xfd, 0xa4, 0xb2, 0x3e, 0xce, 0x67}
DEFINE_GUID!{GUID_MetadataFormat8BIMIPTC,
    0x0010568c, 0x0852, 0x4e6a, 0xb1, 0x91, 0x5c, 0x33, 0xac, 0x5b, 0x04, 0x30}
DEFINE_GUID!{GUID_MetadataFormat8BIMResolutionInfo,
    0x739f305d, 0x81db, 0x43cb, 0xac, 0x5e, 0x55, 0x01, 0x3e, 0xf9, 0xf0, 0x03}
DEFINE_GUID!{GUID_MetadataFormat8BIMIPTCDigest,
    0x1ca32285, 0x9ccd, 0x4786, 0x8b, 0xd8, 0x79, 0x53, 0x9d, 0xb6, 0xa0, 0x06}
DEFINE_GUID!{GUID_MetadataFormatXMP,
    0xbb5acc38, 0xf216, 0x4cec, 0xa6, 0xc5, 0x5f, 0x6e, 0x73, 0x97, 0x63, 0xa9}
DEFINE_GUID!{GUID_MetadataFormatThumbnail,
    0x243dcee9, 0x8703, 0x40ee, 0x8e, 0xf0, 0x22, 0xa6, 0x00, 0xb8, 0x05, 0x8c}
DEFINE_GUID!{GUID_MetadataFormatChunktEXt,
    0x568d8936, 0xc0a9, 0x4923, 0x90, 0x5d, 0xdf, 0x2b, 0x38, 0x23, 0x8f, 0xbc}
DEFINE_GUID!{GUID_MetadataFormatXMPStruct,
    0x22383cf1, 0xed17, 0x4e2e, 0xaf, 0x17, 0xd8, 0x5b, 0x8f, 0x6b, 0x30, 0xd0}
DEFINE_GUID!{GUID_MetadataFormatXMPBag,
    0x833cca5f, 0xdcb7, 0x4516, 0x80, 0x6f, 0x65, 0x96, 0xab, 0x26, 0xdc, 0xe4}
DEFINE_GUID!{GUID_MetadataFormatXMPSeq,
    0x63e8df02, 0xeb6c, 0x456c, 0xa2, 0x24, 0xb2, 0x5e, 0x79, 0x4f, 0xd6, 0x48}
DEFINE_GUID!{GUID_MetadataFormatXMPAlt,
    0x7b08a675, 0x91aa, 0x481b, 0xa7, 0x98, 0x4d, 0xa9, 0x49, 0x08, 0x61, 0x3b}
DEFINE_GUID!{GUID_MetadataFormatLSD,
    0xe256031e, 0x6299, 0x4929, 0xb9, 0x8d, 0x5a, 0xc8, 0x84, 0xaf, 0xba, 0x92}
DEFINE_GUID!{GUID_MetadataFormatIMD,
    0xbd2bb086, 0x4d52, 0x48dd, 0x96, 0x77, 0xdb, 0x48, 0x3e, 0x85, 0xae, 0x8f}
DEFINE_GUID!{GUID_MetadataFormatGCE,
    0x2a25cad8, 0xdeeb, 0x4c69, 0xa7, 0x88, 0x0e, 0xc2, 0x26, 0x6d, 0xca, 0xfd}
DEFINE_GUID!{GUID_MetadataFormatAPE,
    0x2e043dc2, 0xc967, 0x4e05, 0x87, 0x5e, 0x61, 0x8b, 0xf6, 0x7e, 0x85, 0xc3}
DEFINE_GUID!{GUID_MetadataFormatJpegChrominance,
    0xf73d0dcf, 0xcec6, 0x4f85, 0x9b, 0x0e, 0x1c, 0x39, 0x56, 0xb1, 0xbe, 0xf7}
DEFINE_GUID!{GUID_MetadataFormatJpegLuminance,
    0x86908007, 0xedfc, 0x4860, 0x8d, 0x4b, 0x4e, 0xe6, 0xe8, 0x3e, 0x60, 0x58}
DEFINE_GUID!{GUID_MetadataFormatJpegComment,
    0x220e5f33, 0xafd3, 0x474e, 0x9d, 0x31, 0x7d, 0x4f, 0xe7, 0x30, 0xf5, 0x57}
DEFINE_GUID!{GUID_MetadataFormatGifComment,
    0xc4b6e0e0, 0xcfb4, 0x4ad3, 0xab, 0x33, 0x9a, 0xad, 0x23, 0x55, 0xa3, 0x4a}
DEFINE_GUID!{GUID_MetadataFormatChunkgAMA,
    0xf00935a5, 0x1d5d, 0x4cd1, 0x81, 0xb2, 0x93, 0x24, 0xd7, 0xec, 0xa7, 0x81}
DEFINE_GUID!{GUID_MetadataFormatChunkbKGD,
    0xe14d3571, 0x6b47, 0x4dea, 0xb6, 0x0a, 0x87, 0xce, 0x0a, 0x78, 0xdf, 0xb7}
DEFINE_GUID!{GUID_MetadataFormatChunkiTXt,
    0xc2bec729, 0x0b68, 0x4b77, 0xaa, 0x0e, 0x62, 0x95, 0xa6, 0xac, 0x18, 0x14}
DEFINE_GUID!{GUID_MetadataFormatChunkcHRM,
    0x9db3655b, 0x2842, 0x44b3, 0x80, 0x67, 0x12, 0xe9, 0xb3, 0x75, 0x55, 0x6a}
DEFINE_GUID!{GUID_MetadataFormatChunkhIST,
    0xc59a82da, 0xdb74, 0x48a4, 0xbd, 0x6a, 0xb6, 0x9c, 0x49, 0x31, 0xef, 0x95}
DEFINE_GUID!{GUID_MetadataFormatChunkiCCP,
    0xeb4349ab, 0xb685, 0x450f, 0x91, 0xb5, 0xe8, 0x02, 0xe8, 0x92, 0x53, 0x6c}
DEFINE_GUID!{GUID_MetadataFormatChunksRGB,
    0xc115fd36, 0xcc6f, 0x4e3f, 0x83, 0x63, 0x52, 0x4b, 0x87, 0xc6, 0xb0, 0xd9}
DEFINE_GUID!{GUID_MetadataFormatChunktIME,
    0x6b00ae2d, 0xe24b, 0x460a, 0x98, 0xb6, 0x87, 0x8b, 0xd0, 0x30, 0x72, 0xfd}
DEFINE_GUID!{GUID_MetadataFormatDds,
    0x4a064603, 0x8c33, 0x4e60, 0x9c, 0x29, 0x13, 0x62, 0x31, 0x70, 0x2d, 0x08}
DEFINE_GUID!{CLSID_WICUnknownMetadataReader,
    0x699745c2, 0x5066, 0x4b82, 0xa8, 0xe3, 0xd4, 0x04, 0x78, 0xdb, 0xec, 0x8c}
DEFINE_GUID!{CLSID_WICUnknownMetadataWriter,
    0xa09cca86, 0x27ba, 0x4f39, 0x90, 0x53, 0x12, 0x1f, 0xa4, 0xdc, 0x08, 0xfc}
DEFINE_GUID!{CLSID_WICApp0MetadataWriter,
    0xf3c633a2, 0x46c8, 0x498e, 0x8f, 0xbb, 0xcc, 0x6f, 0x72, 0x1b, 0xbc, 0xde}
DEFINE_GUID!{CLSID_WICApp0MetadataReader,
    0x43324b33, 0xa78f, 0x480f, 0x91, 0x11, 0x96, 0x38, 0xaa, 0xcc, 0xc8, 0x32}
DEFINE_GUID!{CLSID_WICApp1MetadataWriter,
    0xee366069, 0x1832, 0x420f, 0xb3, 0x81, 0x04, 0x79, 0xad, 0x06, 0x6f, 0x19}
DEFINE_GUID!{CLSID_WICApp1MetadataReader,
    0xdde33513, 0x774e, 0x4bcd, 0xae, 0x79, 0x02, 0xf4, 0xad, 0xfe, 0x62, 0xfc}
DEFINE_GUID!{CLSID_WICApp13MetadataWriter,
    0x7b19a919, 0xa9d6, 0x49e5, 0xbd, 0x45, 0x02, 0xc3, 0x4e, 0x4e, 0x4c, 0xd5}
DEFINE_GUID!{CLSID_WICApp13MetadataReader,
    0xaa7e3c50, 0x864c, 0x4604, 0xbc, 0x04, 0x8b, 0x0b, 0x76, 0xe6, 0x37, 0xf6}
DEFINE_GUID!{CLSID_WICIfdMetadataReader,
    0x8f914656, 0x9d0a, 0x4eb2, 0x90, 0x19, 0x0b, 0xf9, 0x6d, 0x8a, 0x9e, 0xe6}
DEFINE_GUID!{CLSID_WICIfdMetadataWriter,
    0xb1ebfc28, 0xc9bd, 0x47a2, 0x8d, 0x33, 0xb9, 0x48, 0x76, 0x97, 0x77, 0xa7}
DEFINE_GUID!{CLSID_WICSubIfdMetadataReader,
    0x50d42f09, 0xecd1, 0x4b41, 0xb6, 0x5d, 0xda, 0x1f, 0xda, 0xa7, 0x56, 0x63}
DEFINE_GUID!{CLSID_WICSubIfdMetadataWriter,
    0x8ade5386, 0x8e9b, 0x4f4c, 0xac, 0xf2, 0xf0, 0x00, 0x87, 0x06, 0xb2, 0x38}
DEFINE_GUID!{CLSID_WICExifMetadataReader,
    0xd9403860, 0x297f, 0x4a49, 0xbf, 0x9b, 0x77, 0x89, 0x81, 0x50, 0xa4, 0x42}
DEFINE_GUID!{CLSID_WICExifMetadataWriter,
    0xc9a14cda, 0xc339, 0x460b, 0x90, 0x78, 0xd4, 0xde, 0xbc, 0xfa, 0xbe, 0x91}
DEFINE_GUID!{CLSID_WICGpsMetadataReader,
    0x3697790b, 0x223b, 0x484e, 0x99, 0x25, 0xc4, 0x86, 0x92, 0x18, 0xf1, 0x7a}
DEFINE_GUID!{CLSID_WICGpsMetadataWriter,
    0xcb8c13e4, 0x62b5, 0x4c96, 0xa4, 0x8b, 0x6b, 0xa6, 0xac, 0xe3, 0x9c, 0x76}
DEFINE_GUID!{CLSID_WICInteropMetadataReader,
    0xb5c8b898, 0x0074, 0x459f, 0xb7, 0x00, 0x86, 0x0d, 0x46, 0x51, 0xea, 0x14}
DEFINE_GUID!{CLSID_WICInteropMetadataWriter,
    0x122ec645, 0xcd7e, 0x44d8, 0xb1, 0x86, 0x2c, 0x8c, 0x20, 0xc3, 0xb5, 0x0f}
DEFINE_GUID!{CLSID_WICThumbnailMetadataReader,
    0xfb012959, 0xf4f6, 0x44d7, 0x9d, 0x09, 0xda, 0xa0, 0x87, 0xa9, 0xdb, 0x57}
DEFINE_GUID!{CLSID_WICThumbnailMetadataWriter,
    0xd049b20c, 0x5dd0, 0x44fe, 0xb0, 0xb3, 0x8f, 0x92, 0xc8, 0xe6, 0xd0, 0x80}
DEFINE_GUID!{CLSID_WICIPTCMetadataReader,
    0x03012959, 0xf4f6, 0x44d7, 0x9d, 0x09, 0xda, 0xa0, 0x87, 0xa9, 0xdb, 0x57}
DEFINE_GUID!{CLSID_WICIPTCMetadataWriter,
    0x1249b20c, 0x5dd0, 0x44fe, 0xb0, 0xb3, 0x8f, 0x92, 0xc8, 0xe6, 0xd0, 0x80}
DEFINE_GUID!{CLSID_WICIRBMetadataReader,
    0xd4dcd3d7, 0xb4c2, 0x47d9, 0xa6, 0xbf, 0xb8, 0x9b, 0xa3, 0x96, 0xa4, 0xa3}
DEFINE_GUID!{CLSID_WICIRBMetadataWriter,
    0x5c5c1935, 0x0235, 0x4434, 0x80, 0xbc, 0x25, 0x1b, 0xc1, 0xec, 0x39, 0xc6}
DEFINE_GUID!{CLSID_WIC8BIMIPTCMetadataReader,
    0x0010668c, 0x0801, 0x4da6, 0xa4, 0xa4, 0x82, 0x65, 0x22, 0xb6, 0xd2, 0x8f}
DEFINE_GUID!{CLSID_WIC8BIMIPTCMetadataWriter,
    0x00108226, 0xee41, 0x44a2, 0x9e, 0x9c, 0x4b, 0xe4, 0xd5, 0xb1, 0xd2, 0xcd}
DEFINE_GUID!{CLSID_WIC8BIMResolutionInfoMetadataReader,
    0x5805137a, 0xe348, 0x4f7c, 0xb3, 0xcc, 0x6d, 0xb9, 0x96, 0x5a, 0x05, 0x99}
DEFINE_GUID!{CLSID_WIC8BIMResolutionInfoMetadataWriter,
    0x4ff2fe0e, 0xe74a, 0x4b71, 0x98, 0xc4, 0xab, 0x7d, 0xc1, 0x67, 0x07, 0xba}
DEFINE_GUID!{CLSID_WIC8BIMIPTCDigestMetadataReader,
    0x02805f1e, 0xd5aa, 0x415b, 0x82, 0xc5, 0x61, 0xc0, 0x33, 0xa9, 0x88, 0xa6}
DEFINE_GUID!{CLSID_WIC8BIMIPTCDigestMetadataWriter,
    0x2db5e62b, 0x0d67, 0x495f, 0x8f, 0x9d, 0xc2, 0xf0, 0x18, 0x86, 0x47, 0xac}
DEFINE_GUID!{CLSID_WICPngTextMetadataReader,
    0x4b59afcc, 0xb8c3, 0x408a, 0xb6, 0x70, 0x89, 0xe5, 0xfa, 0xb6, 0xfd, 0xa7}
DEFINE_GUID!{CLSID_WICPngTextMetadataWriter,
    0xb5ebafb9, 0x253e, 0x4a72, 0xa7, 0x44, 0x07, 0x62, 0xd2, 0x68, 0x56, 0x83}
DEFINE_GUID!{CLSID_WICXMPMetadataReader,
    0x72b624df, 0xae11, 0x4948, 0xa6, 0x5c, 0x35, 0x1e, 0xb0, 0x82, 0x94, 0x19}
DEFINE_GUID!{CLSID_WICXMPMetadataWriter,
    0x1765e14e, 0x1bd4, 0x462e, 0xb6, 0xb1, 0x59, 0x0b, 0xf1, 0x26, 0x2a, 0xc6}
DEFINE_GUID!{CLSID_WICXMPStructMetadataReader,
    0x01b90d9a, 0x8209, 0x47f7, 0x9c, 0x52, 0xe1, 0x24, 0x4b, 0xf5, 0x0c, 0xed}
DEFINE_GUID!{CLSID_WICXMPStructMetadataWriter,
    0x22c21f93, 0x7ddb, 0x411c, 0x9b, 0x17, 0xc5, 0xb7, 0xbd, 0x06, 0x4a, 0xbc}
DEFINE_GUID!{CLSID_WICXMPBagMetadataReader,
    0xe7e79a30, 0x4f2c, 0x4fab, 0x8d, 0x00, 0x39, 0x4f, 0x2d, 0x6b, 0xbe, 0xbe}
DEFINE_GUID!{CLSID_WICXMPBagMetadataWriter,
    0xed822c8c, 0xd6be, 0x4301, 0xa6, 0x31, 0x0e, 0x14, 0x16, 0xba, 0xd2, 0x8f}
DEFINE_GUID!{CLSID_WICXMPSeqMetadataReader,
    0x7f12e753, 0xfc71, 0x43d7, 0xa5, 0x1d, 0x92, 0xf3, 0x59, 0x77, 0xab, 0xb5}
DEFINE_GUID!{CLSID_WICXMPSeqMetadataWriter,
    0x6d68d1de, 0xd432, 0x4b0f, 0x92, 0x3a, 0x09, 0x11, 0x83, 0xa9, 0xbd, 0xa7}
DEFINE_GUID!{CLSID_WICXMPAltMetadataReader,
    0xaa94dcc2, 0xb8b0, 0x4898, 0xb8, 0x35, 0x00, 0x0a, 0xab, 0xd7, 0x43, 0x93}
DEFINE_GUID!{CLSID_WICXMPAltMetadataWriter,
    0x076c2a6c, 0xf78f, 0x4c46, 0xa7, 0x23, 0x35, 0x83, 0xe7, 0x08, 0x76, 0xea}
DEFINE_GUID!{CLSID_WICLSDMetadataReader,
    0x41070793, 0x59e4, 0x479a, 0xa1, 0xf7, 0x95, 0x4a, 0xdc, 0x2e, 0xf5, 0xfc}
DEFINE_GUID!{CLSID_WICLSDMetadataWriter,
    0x73c037e7, 0xe5d9, 0x4954, 0x87, 0x6a, 0x6d, 0xa8, 0x1d, 0x6e, 0x57, 0x68}
DEFINE_GUID!{CLSID_WICGCEMetadataReader,
    0xb92e345d, 0xf52d, 0x41f3, 0xb5, 0x62, 0x08, 0x1b, 0xc7, 0x72, 0xe3, 0xb9}
DEFINE_GUID!{CLSID_WICGCEMetadataWriter,
    0xaf95dc76, 0x16b2, 0x47f4, 0xb3, 0xea, 0x3c, 0x31, 0x79, 0x66, 0x93, 0xe7}
DEFINE_GUID!{CLSID_WICIMDMetadataReader,
    0x7447a267, 0x0015, 0x42c8, 0xa8, 0xf1, 0xfb, 0x3b, 0x94, 0xc6, 0x83, 0x61}
DEFINE_GUID!{CLSID_WICIMDMetadataWriter,
    0x8c89071f, 0x452e, 0x4e95, 0x96, 0x82, 0x9d, 0x10, 0x24, 0x62, 0x71, 0x72}
DEFINE_GUID!{CLSID_WICAPEMetadataReader,
    0x1767b93a, 0xb021, 0x44ea, 0x92, 0x0f, 0x86, 0x3c, 0x11, 0xf4, 0xf7, 0x68}
DEFINE_GUID!{CLSID_WICAPEMetadataWriter,
    0xbd6edfca, 0x2890, 0x482f, 0xb2, 0x33, 0x8d, 0x73, 0x39, 0xa1, 0xcf, 0x8d}
DEFINE_GUID!{CLSID_WICJpegChrominanceMetadataReader,
    0x50b1904b, 0xf28f, 0x4574, 0x93, 0xf4, 0x0b, 0xad, 0xe8, 0x2c, 0x69, 0xe9}
DEFINE_GUID!{CLSID_WICJpegChrominanceMetadataWriter,
    0x3ff566f0, 0x6e6b, 0x49d4, 0x96, 0xe6, 0xb7, 0x88, 0x86, 0x69, 0x2c, 0x62}
DEFINE_GUID!{CLSID_WICJpegLuminanceMetadataReader,
    0x356f2f88, 0x05a6, 0x4728, 0xb9, 0xa4, 0x1b, 0xfb, 0xce, 0x04, 0xd8, 0x38}
DEFINE_GUID!{CLSID_WICJpegLuminanceMetadataWriter,
    0x1d583abc, 0x8a0e, 0x4657, 0x99, 0x82, 0xa3, 0x80, 0xca, 0x58, 0xfb, 0x4b}
DEFINE_GUID!{CLSID_WICJpegCommentMetadataReader,
    0x9f66347c, 0x60c4, 0x4c4d, 0xab, 0x58, 0xd2, 0x35, 0x86, 0x85, 0xf6, 0x07}
DEFINE_GUID!{CLSID_WICJpegCommentMetadataWriter,
    0xe573236f, 0x55b1, 0x4eda, 0x81, 0xea, 0x9f, 0x65, 0xdb, 0x02, 0x90, 0xd3}
DEFINE_GUID!{CLSID_WICGifCommentMetadataReader,
    0x32557d3b, 0x69dc, 0x4f95, 0x83, 0x6e, 0xf5, 0x97, 0x2b, 0x2f, 0x61, 0x59}
DEFINE_GUID!{CLSID_WICGifCommentMetadataWriter,
    0xa02797fc, 0xc4ae, 0x418c, 0xaf, 0x95, 0xe6, 0x37, 0xc7, 0xea, 0xd2, 0xa1}
DEFINE_GUID!{CLSID_WICPngGamaMetadataReader,
    0x3692ca39, 0xe082, 0x4350, 0x9e, 0x1f, 0x37, 0x04, 0xcb, 0x08, 0x3c, 0xd5}
DEFINE_GUID!{CLSID_WICPngGamaMetadataWriter,
    0xff036d13, 0x5d4b, 0x46dd, 0xb1, 0x0f, 0x10, 0x66, 0x93, 0xd9, 0xfe, 0x4f}
DEFINE_GUID!{CLSID_WICPngBkgdMetadataReader,
    0x0ce7a4a6, 0x03e8, 0x4a60, 0x9d, 0x15, 0x28, 0x2e, 0xf3, 0x2e, 0xe7, 0xda}
DEFINE_GUID!{CLSID_WICPngBkgdMetadataWriter,
    0x68e3f2fd, 0x31ae, 0x4441, 0xbb, 0x6a, 0xfd, 0x70, 0x47, 0x52, 0x5f, 0x90}
DEFINE_GUID!{CLSID_WICPngItxtMetadataReader,
    0xaabfb2fa, 0x3e1e, 0x4a8f, 0x89, 0x77, 0x55, 0x56, 0xfb, 0x94, 0xea, 0x23}
DEFINE_GUID!{CLSID_WICPngItxtMetadataWriter,
    0x31879719, 0xe751, 0x4df8, 0x98, 0x1d, 0x68, 0xdf, 0xf6, 0x77, 0x04, 0xed}
DEFINE_GUID!{CLSID_WICPngChrmMetadataReader,
    0xf90b5f36, 0x367b, 0x402a, 0x9d, 0xd1, 0xbc, 0x0f, 0xd5, 0x9d, 0x8f, 0x62}
DEFINE_GUID!{CLSID_WICPngChrmMetadataWriter,
    0xe23ce3eb, 0x5608, 0x4e83, 0xbc, 0xef, 0x27, 0xb1, 0x98, 0x7e, 0x51, 0xd7}
DEFINE_GUID!{CLSID_WICPngHistMetadataReader,
    0x877a0bb7, 0xa313, 0x4491, 0x87, 0xb5, 0x2e, 0x6d, 0x05, 0x94, 0xf5, 0x20}
DEFINE_GUID!{CLSID_WICPngHistMetadataWriter,
    0x8a03e749, 0x672e, 0x446e, 0xbf, 0x1f, 0x2c, 0x11, 0xd2, 0x33, 0xb6, 0xff}
DEFINE_GUID!{CLSID_WICPngIccpMetadataReader,
    0xf5d3e63b, 0xcb0f, 0x4628, 0xa4, 0x78, 0x6d, 0x82, 0x44, 0xbe, 0x36, 0xb1}
DEFINE_GUID!{CLSID_WICPngIccpMetadataWriter,
    0x16671e5f, 0x0ce6, 0x4cc4, 0x97, 0x68, 0xe8, 0x9f, 0xe5, 0x01, 0x8a, 0xde}
DEFINE_GUID!{CLSID_WICPngSrgbMetadataReader,
    0xfb40360c, 0x547e, 0x4956, 0xa3, 0xb9, 0xd4, 0x41, 0x88, 0x59, 0xba, 0x66}
DEFINE_GUID!{CLSID_WICPngSrgbMetadataWriter,
    0xa6ee35c6, 0x87ec, 0x47df, 0x9f, 0x22, 0x1d, 0x5a, 0xad, 0x84, 0x0c, 0x82}
DEFINE_GUID!{CLSID_WICPngTimeMetadataReader,
    0xd94edf02, 0xefe5, 0x4f0d, 0x85, 0xc8, 0xf5, 0xa6, 0x8b, 0x30, 0x00, 0xb1}
DEFINE_GUID!{CLSID_WICPngTimeMetadataWriter,
    0x1ab78400, 0xb5a3, 0x4d91, 0x8a, 0xce, 0x33, 0xfc, 0xd1, 0x49, 0x9b, 0xe6}
DEFINE_GUID!{CLSID_WICDdsMetadataReader,
    0x276c88ca, 0x7533, 0x4a86, 0xb6, 0x76, 0x66, 0xb3, 0x60, 0x80, 0xd4, 0x84}
DEFINE_GUID!{CLSID_WICDdsMetadataWriter,
    0xfd688bbd, 0x31ed, 0x4db7, 0xa7, 0x23, 0x93, 0x49, 0x27, 0xd3, 0x83, 0x67}
ENUM!{enum WICMetadataCreationOptions {
    WICMetadataCreationDefault = 0,
    WICMetadataCreationAllowUnknown = WICMetadataCreationDefault,
    WICMetadataCreationFailUnknown = 0x10000,
    WICMetadataCreationMask = 0xffff0000,
}}
ENUM!{enum WICPersistOptions {
    WICPersistOptionDefault = 0,
    WICPersistOptionLittleEndian = 0,
    WICPersistOptionBigEndian = 0x1,
    WICPersistOptionStrictFormat = 0x2,
    WICPersistOptionNoCacheStream = 0x4,
    WICPersistOptionPreferUTF8 = 0x8,
    WICPersistOptionMask = 0xffff,
}}
RIDL!(#[uuid(0xfeaa2a8d, 0xb3f3, 0x43e4, 0xb2, 0x5c, 0xd1, 0xde, 0x99, 0x0a, 0x1a, 0xe1)]
interface IWICMetadataBlockReader(IWICMetadataBlockReaderVtbl): IUnknown(IUnknownVtbl) {
    fn GetContainerFormat(
        pguidContainerFormat: *mut GUID,
    ) -> HRESULT,
    fn GetCount(
        pcCount: *mut UINT,
    ) -> HRESULT,
    fn GetReaderByIndex(
        ppIMetadataReader: *mut *mut IWICMetadataReader,
    ) -> HRESULT,
    fn GetEnumerator(
        ppIEnumMetadata: *mut IEnumUnknown,
    ) -> HRESULT,
});
RIDL!(#[uuid(0x08fb9676, 0xb444, 0x41e8, 0x8d, 0xbe, 0x6a, 0x53, 0xa5, 0x42, 0xbf, 0xf1)]
interface IWICMetadataBlockWriter(IWICMetadataBlockWriterVtbl):
    IWICMetadataBlockReader(IWICMetadataBlockReaderVtbl) {
    fn InitializeFromBlockReader(
        pIMDBlockReader: *mut IWICMetadataBlockReader,
    ) -> HRESULT,
    fn GetWriterByIndex(
        ppIMetadataWriter: *mut *mut IWICMetadataWriter,
    ) -> HRESULT,
    fn AddWriter(
        pIMetadataWriter: *mut IWICMetadataWriter,
    ) -> HRESULT,
    fn SetWriterByIndex(
        pIMetadataWriter: *mut IWICMetadataWriter,
    ) -> HRESULT,
    fn RemoveWriterByIndex(
        nIndex: UINT,
    ) -> HRESULT,
});
RIDL!(#[uuid(0x9204fe99, 0xd8fc, 0x4fd5, 0xa0, 0x01, 0x95, 0x36, 0xb0, 0x67, 0xa8, 0x99)]
interface IWICMetadataReader(IWICMetadataReaderVtbl): IUnknown(IUnknownVtbl) {
    fn GetMetadataFormat(
        pguidMetadataFormat: *mut GUID,
    ) -> HRESULT,
    fn GetMetadataHandlerInfo(
        ppIHandler: *mut *mut IWICMetadataHandlerInfo,
    ) -> HRESULT,
    fn GetCount(
        pcCount: *mut UINT,
    ) -> HRESULT,
    fn GetValueByIndex(
        nIndex: UINT,
        pvarSchema: *mut PROPVARIANT,
        pvarId: *mut PROPVARIANT,
        pvarValue: *mut PROPVARIANT,
    ) -> HRESULT,
    fn GetValue(
        pvarSchema: *const PROPVARIANT,
        pvarId: *const PROPVARIANT,
        pvarValue: *mut PROPVARIANT,
    ) -> HRESULT,
    fn GetEnumerator(
        ppIEnumMetadata: *mut *mut IWICEnumMetadataItem,
    ) -> HRESULT,
});
RIDL!(#[uuid(0xf7836e16, 0x3be0, 0x470b, 0x86, 0xbb, 0x16, 0x0d, 0x0a, 0xec, 0xd7, 0xde)]
interface IWICMetadataWriter(IWICMetadataWriterVtbl): IWICMetadataReader(IWICMetadataReaderVtbl) {
    fn SetValue(
        pvarSchema: *const PROPVARIANT,
        pvarId: *const PROPVARIANT,
        pvarValue: *const PROPVARIANT,
    ) -> HRESULT,
    fn SetValueByIndex(
        nIndex: UINT,
        pvarSchema: *const PROPVARIANT,
        pvarId: *const PROPVARIANT,
        pvarValue: *const PROPVARIANT,
    ) -> HRESULT,
    fn RemoveValue(
        pvarSchema: *const PROPVARIANT,
        pvarId: *const PROPVARIANT,
    ) -> HRESULT,
    fn RemoveValueByIndex(
        nIndex: UINT,
    ) -> HRESULT,
});
RIDL!(#[uuid(0x449494bc, 0xb468, 0x4927, 0x96, 0xd7, 0xba, 0x90, 0xd3, 0x1a, 0xb5, 0x05)]
interface IWICStreamProvider(IWICStreamProviderVtbl): IUnknown(IUnknownVtbl) {
    fn GetStream(
        ppIStream: *mut *mut IStream,
    ) -> HRESULT,
    fn GetPersistOptions(
        pdwPersistOptions: *mut DWORD,
    ) -> HRESULT,
    fn GetPreferredVendorGUID(
        pguidPreferredVendor: *mut GUID,
    ) -> HRESULT,
    fn RefreshStream() -> HRESULT,
});
RIDL!(#[uuid(0x00675040, 0x6908, 0x45f8, 0x86, 0xa3, 0x49, 0xc7, 0xdf, 0xd6, 0xd9, 0xad)]
interface IWICPersistStream(IWICPersistStreamVtbl): IPersistStream(IPersistStreamVtbl) {
    fn LoadEx(
        pIStream: *mut IStream,
        pguidPreferredVendor: *const GUID,
        dwPersistOptions: DWORD,
    ) -> HRESULT,
    fn SaveEx(
        pIStream: *mut IStream,
        dwPersistOptions: DWORD,
        fClearDirty: BOOL,
    ) -> HRESULT,
});
RIDL!(#[uuid(0xaba958bf, 0xc672, 0x44d1, 0x8d, 0x61, 0xce, 0x6d, 0xf2, 0xe6, 0x82, 0xc2)]
interface IWICMetadataHandlerInfo(IWICMetadataHandlerInfoVtbl):
    IWICComponentInfo(IWICComponentInfoVtbl) {
    fn GetMetadataFormat(
        pguidMetadataFormat: *mut GUID,
    ) -> HRESULT,
    fn GetContainerFormats(
        cContainerFormats: UINT,
        pguidContainerFormats: *mut GUID,
        pcchActual: *mut UINT,
    ) -> HRESULT,
    fn GetDeviceManufacturer(
        cchDeviceManufacturer: UINT,
        wzDeviceManufacturer: *mut WCHAR,
        pcchActual: *mut UINT,
    ) -> HRESULT,
    fn GetDeviceModels(
        cchDeviceModels: UINT,
        wzDeviceModels: *mut WCHAR,
        pcchActual: *mut UINT,
    ) -> HRESULT,
    fn DoesRequireFullStream(
        pfRequiresFullStream: *mut BOOL,
    ) -> HRESULT,
    fn DoesSupportPadding(
        pfSupportsPadding: *mut BOOL,
    ) -> HRESULT,
    fn DoesRequireFixedSize(
        pfFixedSize: *mut BOOL,
    ) -> HRESULT,
});
STRUCT!{struct WICMetadataPattern {
    Position: ULARGE_INTEGER,
    Length: ULONG,
    Pattern: *mut BYTE,
    Mask: *mut BYTE,
    DataOffset: ULARGE_INTEGER,
}}
RIDL!(#[uuid(0xeebf1f5b, 0x07c1, 0x4447, 0xa3, 0xab, 0x22, 0xac, 0xaf, 0x78, 0xa8, 0x04)]
interface IWICMetadataReaderInfo(IWICMetadataReaderInfoVtbl):
    IWICMetadataHandlerInfo(IWICMetadataHandlerInfoVtbl) {
    fn GetPatterns(
        guidContainerFormat: REFGUID,
        cbSize: UINT,
        pPattern: *mut WICMetadataPattern,
        pcCount: *mut UINT,
        pcbActual: *mut UINT,
    ) -> HRESULT,
    fn MatchesPattern(
        guidContainerFormat: REFGUID,
        pIStream: *mut IStream,
        pfMatches: *mut BOOL,
    ) -> HRESULT,
    fn CreateInstance(
        ppIReader: *mut *mut IWICMetadataReader,
    ) -> HRESULT,
});
STRUCT!{struct WICMetadataHeader {
    Position: ULARGE_INTEGER,
    Length: ULONG,
    Header: *mut BYTE,
    DataOffset: ULARGE_INTEGER,
}}
RIDL!(#[uuid(0xb22e3fba, 0x3925, 0x4323, 0xb5, 0xc1, 0x9e, 0xbf, 0xc4, 0x30, 0xf2, 0x36)]
interface IWICMetadataWriterInfo(IWICMetadataWriterInfoVtbl):
    IWICMetadataHandlerInfo(IWICMetadataHandlerInfoVtbl) {
    fn GetHeader(
        guidContainerFormat: REFGUID,
        cbSize: UINT,
        pHeader: *mut WICMetadataHeader,
        pcbActual: *mut UINT,
    ) -> HRESULT,
    fn CreateInstance(
        ppIWriter: *mut *mut IWICMetadataWriter,
    ) -> HRESULT,
});
RIDL!(#[uuid(0x412d0c3a, 0x9650, 0x44fa, 0xaf, 0x5b, 0xdd, 0x2a, 0x06, 0xc8, 0xe8, 0xfb)]
interface IWICComponentFactory(IWICComponentFactoryVtbl):
    IWICImagingFactory(IWICImagingFactoryVtbl) {
    fn CreateMetadataReader(
        guidMetadataFormat: REFGUID,
        pguidVendor: *const GUID,
        dwOptions: DWORD,
        pIStream: *mut IStream,
        ppIReader: *mut *mut IWICMetadataReader,
    ) -> HRESULT,
    fn CreateMetadataReaderFromContainer(
        guidContainerFormat: REFGUID,
        pguidVendor: *const GUID,
        dwOptions: DWORD,
        pIStream: *mut IStream,
        ppIReader: *mut *mut IWICMetadataReader,
    ) -> HRESULT,
    fn CreateMetadataWriter(
        guidMetadataFormat: REFGUID,
        pguidVendor: *const GUID,
        dwMetadataOptions: DWORD,
        ppIWriter: *mut *mut IWICMetadataWriter,
    ) -> HRESULT,
    fn CreateMetadataWriterFromReader(
        pIReader: *mut IWICMetadataReader,
        pguidVendor: *const GUID,
        ppIWriter: *mut *mut IWICMetadataWriter,
    ) -> HRESULT,
    fn CreateQueryReaderFromBlockReader(
        pIBlockReader: *mut IWICMetadataBlockReader,
        ppIQueryReader: *mut *mut IWICMetadataQueryReader,
    ) -> HRESULT,
    fn CreateQueryWriterFromBlockWriter(
        pIBlockWriter: *mut IWICMetadataBlockWriter,
        ppIQueryWriter: *mut *mut IWICMetadataQueryWriter,
    ) -> HRESULT,
    fn CreateEncoderPropertyBag(
        ppropOptions: *mut PROPBAG2,
        cCount: UINT,
        ppIPropertyBag: *mut *mut IPropertyBag2,
    ) -> HRESULT,
});
extern "system" {
    pub fn WICMatchMetadataContent(
        guidContainerFormat: REFGUID,
        pguidVendor: *const GUID,
        pIStream: *mut IStream,
        pguidMetadataFormat: *mut GUID,
    ) -> HRESULT;
    pub fn WICSerializeMetadataContent(
        guidContainerFormat: REFGUID,
        pIWriter: *mut IWICMetadataWriter,
        dwPersistOptions: DWORD,
        pIStream: *mut IStream,
    ) -> HRESULT;
    pub fn WICGetMetadataContentSize(
        guidContainerFormat: REFGUID,
        pIWriter: *mut IWICMetadataWriter,
        pcbSize: *mut ULARGE_INTEGER,
    ) -> HRESULT;
    pub fn BSTR_UserSize(
        pFlags: *mut c_ulong,
        Offset: c_ulong,
        pBstr: *mut BSTR,
    ) -> c_ulong;
    pub fn BSTR_UserMarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pBstr: *mut BSTR,
    ) -> *mut c_uchar;
    pub fn BSTR_UserUnmarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pBstr: *mut BSTR,
    ) -> *mut c_uchar;
    pub fn BSTR_UserFree(
        pFlags: *mut c_ulong,
        pBstr: *mut BSTR,
    );
    pub fn CLIPFORMAT_UserSize(
        pFlags: *mut c_ulong,
        Offset: c_ulong,
        pCF: *mut CLIPFORMAT,
    ) -> c_ulong;
    pub fn CLIPFORMAT_UserMarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pCF: *mut CLIPFORMAT,
    ) -> *mut c_uchar;
    pub fn CLIPFORMAT_UserUnmarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pCF: *mut CLIPFORMAT,
    ) -> *mut c_uchar;
    pub fn CLIPFORMAT_UserFree(
        pFlags: *mut c_ulong,
        pCF: *mut CLIPFORMAT,
    );
    pub fn LPSAFEARRAY_UserSize(
        pFlags: *mut c_ulong,
        Offset: c_ulong,
        phBmp: *mut LPSAFEARRAY,
    ) -> c_ulong;
    pub fn LPSAFEARRAY_UserMarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pBstr: *mut LPSAFEARRAY,
    ) -> *mut c_uchar;
    pub fn LPSAFEARRAY_UserUnmarshal(
        pFlags: *mut c_ulong,
        pBuffer: *mut c_uchar,
        pBstr: *mut LPSAFEARRAY,
    ) -> *mut c_uchar;
    pub fn LPSAFEARRAY_UserFree(
        pFlags: *mut c_ulong,
        pBstr: *mut LPSAFEARRAY,
    );
}

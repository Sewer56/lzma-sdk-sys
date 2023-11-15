// Always first!!
#include "7z/C/Precomp.h"

// Base types and common functionality
#include "7z/C/7zTypes.h"

// Memory and system
#include "7z/C/Alloc.h"
#include "7z/C/Threads.h"
#include "7z/C/CpuArch.h"

// Core compression codecs
#include "7z/C/LzmaDec.h"
#include "7z/C/LzmaEnc.h"
#include "7z/C/LzFind.h"
#include "7z/C/LzFindMt.h"
#include "7z/C/Lzma2Dec.h"
#include "7z/C/Lzma2Enc.h"
#include "7z/C/LzmaLib.h"
#include "7z/C/Lzma86.h"
#include "7z/C/Ppmd7.h"
#include "7z/C/Ppmd8.h"
#include "7z/C/Ppmd.h"

// Filters and transforms
#include "7z/C/Delta.h"
#include "7z/C/Bcj2.h"
#include "7z/C/Bra.h"
#include "7z/C/BwtSort.h"

// Checksum and cryptography
#include "7z/C/7zCrc.h"
#include "7z/C/Aes.h"
#include "7z/C/Sha1.h"
#include "7z/C/Sha256.h"
#include "7z/C/Blake2.h"

// Archive format support
#include "7z/C/7z.h"
#include "7z/C/7zFile.h"

// XZ format support
#include "7z/C/Xz.h"
#include "7z/C/XzCrc64.h"
#include "7z/C/XzEnc.h"

// Other compression formats
#include "7z/C/ZstdDec.h"
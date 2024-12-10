// Always first!!
#include "7z/C/Precomp.h"

// LZMA Encode/Decode Functionality
#include "7z/C/LzmaDec.h"
#include "7z/C/LzmaEnc.h"
#include "7z/C/Lzma2Dec.h"
#include "7z/C/Lzma2Enc.h"
#include "7z/C/LzFind.h"
#include "7z/C/LzFindMt.h"
#include "7z/C/LzFindOpt.c" // .c is not a typo. There is no header.

// Threading for the multithreaded logic
#include "7z/C/Threads.h"
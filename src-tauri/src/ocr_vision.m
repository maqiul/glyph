// macOS 原生 OCR：Apple Vision 框架（VNRecognizeTextRequest）。
// 由 build.rs 用 cc 编译（-fobjc-arc），并链接 Vision/CoreGraphics/ImageIO/Foundation。
// 供 ocr_vision.rs 通过 extern "C" 调用。

#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>

// 识别图片字节（png/jpg/bmp/webp/...），返回换行拼接的 UTF-8 文本。
// 成功：返回 malloc 的字符串（调用方用 glyph_ocr_free 释放），*out_error 置空。
// 失败：返回 NULL，并把错误信息写入 *out_error（同样用 glyph_ocr_free 释放）。
const char *glyph_ocr_recognize(const unsigned char *data, NSUInteger len,
                                char **out_error) {
    @autoreleasepool {
        if (out_error) *out_error = NULL;
        if (data == NULL || len == 0) {
            if (out_error) *out_error = strdup("empty image data");
            return NULL;
        }

        NSData *imgData = [NSData dataWithBytes:data length:len];
        CGImageSourceRef src =
            CGImageSourceCreateWithData((__bridge CFDataRef)imgData, NULL);
        if (!src) {
            if (out_error) *out_error = strdup("CGImageSourceCreateWithData failed");
            return NULL;
        }
        CGImageRef cgImage = CGImageSourceCreateImageAtIndex(src, 0, NULL);
        CFRelease(src);
        if (!cgImage) {
            if (out_error) *out_error = strdup("image decode failed");
            return NULL;
        }

        VNImageRequestHandler *handler =
            [[VNImageRequestHandler alloc] initWithCGImage:cgImage options:@{}];
        CGImageRelease(cgImage);

        VNRecognizeTextRequest *request =
            [[VNRecognizeTextRequest alloc] initWithCompletionHandler:nil];
        request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
        request.recognitionLanguages = @[@"zh-Hans", @"en-US"];
        request.usesLanguageCorrection = YES;

        NSError *err = nil;
        BOOL ok = [handler performRequests:@[ request ] error:&err];
        if (!ok) {
            NSString *msg = err.localizedDescription ?: @"Vision performRequests failed";
            if (out_error) *out_error = strdup(msg.UTF8String);
            return NULL;
        }

        NSMutableString *result = [NSMutableString string];
        NSCharacterSet *ws = [NSCharacterSet whitespaceCharacterSet];
        for (VNRecognizedTextObservation *obs in request.results) {
            VNRecognizedText *top = [[obs topCandidates:1] firstObject];
            if (!top) continue;
            NSString *line = [top.string stringByTrimmingCharactersInSet:ws];
            if (line.length > 0) {
                if (result.length > 0) [result appendString:@"\n"];
                [result appendString:line];
            }
        }
        return strdup(result.UTF8String);
    }
}

void glyph_ocr_free(char *ptr) {
    if (ptr != NULL) free(ptr);
}

/**
Package mjpeg contains an MJPEG video format writer. This package was converted manually from https://github.com/icza/mjpeg

Examples:

Let's see an example how to turn the JPEG files 1.jpg, 2.jpg, ..., 10.jpg into a movie file:

    checkErr := func(err error) {
        if err != nil {
            panic(err)
        }
    }

    // Video size: 200x100 pixels, FPS: 2
    aw, err := mjpeg.New("test.avi", 200, 100, 2)
    checkErr(err)

    // Create a movie from images: 1.jpg, 2.jpg, ..., 10.jpg
    for i := 1; i <= 10; i++ {
        data, err := ioutil.ReadFile(fmt.Sprintf("%d.jpg", i))
        checkErr(err)
        checkErr(aw.AddFrame(data))
    }

    checkErr(aw.Close())

Example to add an image.Image as a frame to the video:

    aw, err := mjpeg.New("test.avi", 200, 100, 2)
    checkErr(err)

    var img image.Image
    // Acquire / initialize image, e.g.:
    // img = image.NewRGBA(image.Rect(0, 0, 200, 100))

    buf := &bytes.Buffer{}
    checkErr(jpeg.Encode(buf, img, nil))
    checkErr(aw.AddFrame(buf.Bytes()))

    checkErr(aw.Close())
*/

use std::fs::File;
use std::io::{Write, Seek};
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn err(s: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(String::from(s))
}

/// AviWriter is an *.avi video writer.
/// The video codec is MJPEG.
pub struct AviWriter {
	// aviFile is the name of the file to write the result to
	avi_file: String,
	// width is the width of the video
	width: u32,
	// height is the height of the video
	height: u32,
	// fps is the frames/second (the "speed") of the video
	fps: u32,

	// avif is the avi file descriptor
	avif: File,
	// idxFile is the name of the index file
	idx: Vec<u8>,
	// idxf is the index file descriptor
	// idxf *os.File

	// lengthFields contains the file positions of the length fields
	// that are filled later; used as a stack (LIFO)
	length_fields: Vec<u64>,

	// Position of the frames count fields
	frames_count_field_pos: u64,
    frames_count_field_pos2: u64,
	// Position of the MOVI chunk
	movi_pos: u64,

	// frames is the number of frames written to the AVI file
	frames: u32,
}

impl AviWriter {
    // New returns a new AviWriter.
    // The Close() method of the AviWriter must be called to finalize the video file.
    pub fn new(avi_file: &str, width: u32, height: u32, fps: u32) -> Result<AviWriter> {
        let mut aw = AviWriter{
            avi_file:     String::from(avi_file),
            width:        width,
            height:       height,
            fps:          fps,
            idx:          vec![],
            length_fields: vec![],
            avif: File::create(avi_file)?,
            frames: 0,
            movi_pos: 0,
            frames_count_field_pos: 0,
            frames_count_field_pos2: 0,
        };

        // Write AVI header
        aw.write_str("RIFF")?;          // RIFF type
        aw.write_length_field()?;       // File length (remaining bytes after this field) (nesting level 0)
        aw.write_str("AVI ")?;          // AVI signature
        aw.write_str("LIST")?;          // LIST chunk: data encoding
        aw.write_length_field()?;               // Chunk length (nesting level 1)
        aw.write_str("hdrl")?;          // LIST chunk type
        aw.write_str("avih")?;          // avih sub-chunk
        aw.write_u32(0x38)?;          // Sub-chunk length excluding the first 8 bytes of avih signature and size
        aw.write_u32(1000000 / fps)?; // Frame delay time in microsec
        aw.write_u32(0)?;             // dwMaxBytesPerSec (maximum data rate of the file in bytes per second)
        aw.write_u32(0)?;             // Reserved
        aw.write_u32(0x10)?;          // dwFlags, 0x10 bit: AVIF_HASINDEX (the AVI file has an index chunk at the end of the file - for good performance); Windows Media Player can't even play it if index is missing!
        aw.frames_count_field_pos = aw.tell()?;
        aw.write_u32(0)?;      // Number of frames
        aw.write_u32(0)?;      // Initial frame for non-interleaved files; non interleaved files should set this to 0
        aw.write_u32(1)?;      // Number of streams in the video; here 1 video, no audio
        aw.write_u32(0)?;      // dwSuggestedBufferSize
        aw.write_u32(width)?;  // Image width in pixels
        aw.write_u32(height)?; // Image height in pixels
        aw.write_u32(0)?;      // Reserved
        aw.write_u32(0)?;
        aw.write_u32(0)?;
        aw.write_u32(0)?;

        // Write stream information
        aw.write_str("LIST")?; // LIST chunk: stream headers
        aw.write_length_field()?;      // Chunk size (nesting level 2)
        aw.write_str("strl")?; // LIST chunk type: stream list
        aw.write_str("strh")?; // Stream header
        aw.write_u32(56)?;   // Length of the strh sub-chunk
        aw.write_str("vids")?; // fccType - type of data stream - here 'vids' for video stream
        aw.write_str("MJPG")?; // MJPG for Motion JPEG
        aw.write_u32(0)?;    // dwFlags
        aw.write_u32(0)?;    // wPriority, wLanguage
        aw.write_u32(0)?;    // dwInitialFrames
        aw.write_u32(1)?;    // dwScale
        aw.write_u32(fps)?;  // dwRate, Frame rate for video streams (the actual FPS is calculated by dividing this by dwScale)
        aw.write_u32(0)?;    // usually zero
        aw.frames_count_field_pos2 = aw.tell()?;
        aw.write_u32(0)?;  // dwLength, playing time of AVI file as defined by scale and rate (set equal to the number of frames)
        aw.write_u32(0)?;  // dwSuggestedBufferSize for reading the stream (typically, this contains a value corresponding to the largest chunk in a stream)
        aw.write_u32(!0)?; // dwQuality, encoding quality given by an integer between (0 and 10,000.  If set to -1, drivers use the default quality value)
        aw.write_u32(0)?;  // dwSampleSize, 0 means that each frame is in its own chunk
        aw.write_u16(0)?;  // left of rcFrame if stream has a different size than dwWidth*dwHeight(unused)
        aw.write_u16(0)?;  //   ..top
        aw.write_u16(0)?;  //   ..right
        aw.write_u16(0)?;  //   ..bottom
        // end of 'strh' chunk, stream format follows
        aw.write_str("strf")?;               // stream format chunk
        aw.write_length_field()?;                    // Chunk size (nesting level 3)
        aw.write_u32(40)?;                 // biSize, write header size of BITMAPINFO header structure; applications should use this size to determine which BITMAPINFO header structure is being used, this size includes this biSize field
        aw.write_u32(width)?;              // biWidth, width in pixels
        aw.write_u32(height)?;             // biWidth, height in pixels (may be negative for uncompressed video to indicate vertical flip)
        aw.write_u16(1)?;                  // biPlanes, number of color planes in which the data is stored
        aw.write_u16(24)?;                 // biBitCount, number of bits per pixel #
        aw.write_str("MJPG")?;               // biCompression, type of compression used (uncompressed: NO_COMPRESSION=0)
        aw.write_u32(width * height * 3)?; // biSizeImage (buffer size for decompressed mage) may be 0 for uncompressed data
        aw.write_u32(0)?;                  // biXPelsPerMeter, horizontal resolution in pixels per meter
        aw.write_u32(0)?;                  // biYPelsPerMeter, vertical resolution in pixels per meter
        aw.write_u32(0)?;                  // biClrUsed (color table size; for 8-bit only)
        aw.write_u32(0)?;                  // biClrImportant, specifies that the first x colors of the color table (0: all the colors are important, or, rather, their relative importance has not been computed)
        aw.finalize_length_field()?;          //'strf' chunk finished (nesting level 3)

        aw.write_str("strn")?; // Use 'strn' to provide a zero terminated text string describing the stream
        let mut name = String::from("Created with https://github.com/icza/mjpeg"); // TODO: + " at " + time.Now().Format("2006-01-02 15:04:05 MST")
        // Name must be 0-terminated and stream name length (the length of the chunk) must be even
        if name.len()&0x01 == 0 {
            name = name + " \000" // padding space plus terminating 0
        } else {
            name = name + "\000" // terminating 0
        }
        aw.write_u32(name.len() as u32)?; // Length of the strn sub-CHUNK (must be even)
        aw.write_str(&name)?;
        aw.finalize_length_field()?; // LIST 'strl' finished (nesting level 2)
        aw.finalize_length_field()?; // LIST 'hdrl' finished (nesting level 1)

        aw.write_str("LIST")?; // The second LIST chunk, which contains the actual data
        aw.write_length_field()?;      // Chunk length (nesting level 1)
        aw.movi_pos = aw.tell()?;
        aw.write_str("movi")?; // LIST chunk type: 'movi'

        Ok(aw)
    }

    // write_str writes a string to the file.
    fn write_str(&mut self, s: &str) -> Result<()> {
        self.avif.write(s.as_bytes())?;
        Ok(())
    }

    // writeInt32 writes a 32-bit int value to the file.
    fn write_u32(&mut self, n: u32) -> Result<()> {
        let mut buf: [u8;4] = [ 0, 0, 0, 0];
        buf[3] = (n & 0xff) as u8;
        buf[2] = ((n >> 8) & 0xff) as u8;
        buf[1] = ((n >> 16) & 0xff) as u8;
        buf[0] = ((n >> 24) & 0xff) as u8;

        self.avif.write(&buf)?;

        Ok(())
    }

    fn idx_u32(&mut self, n: u32) {
        self.idx.push((n & 0xff) as u8);
        self.idx.push(((n >> 8) & 0xff) as u8);
        self.idx.push(((n >> 16) & 0xff) as u8);
        self.idx.push(((n >> 24) & 0xff) as u8);
    }

    // write_u16 writes a 16-bit int value to the index file.
    fn write_u16(&mut self, n: u16) -> Result<()> {
        let mut buf: [u8;2] = [0, 0];
        buf[1] = (n & 0xff) as u8;
        buf[0] = ((n >> 8) & 0xff) as u8;

        self.avif.write(&buf)?;

        Ok(())
    }

    // writeLengthField writes an empty int field to the avi file, and saves
    // the current file position as it will be filled later.
    fn write_length_field(&mut self) -> Result<()> {
        let pos = self.tell()?;
        self.length_fields.push(pos);
        self.write_u32(0)?;

        Ok(())
    }

    // finalizeLengthField finalizes the last length field.
    fn finalize_length_field(&mut self) -> Result<()> {
        let pos = self.tell()?;
        let len_pos = match self.length_fields.pop() {
            Some(l) => l,
            None => {
                return Err(err("Internal error"));
            }
        };

        self.avif.seek(std::io::SeekFrom::Start(len_pos))?;
        self.write_u32((pos - len_pos - 4) as u32)?;
        self.avif.seek(std::io::SeekFrom::Start(pos))?;
        if pos % 2 == 1 {
            self.avif.write(&[0])?;
        }
        Ok(())
    }

    fn tell(&mut self) -> Result<u64> {
        Ok(self.avif.seek(std::io::SeekFrom::Current(0))?)
    }

    /// add_frame adds new frame to MJpeg stream
    pub fn add_frame(&mut self, jpeg_data: &[u8]) -> Result<()> {
        let frame_pos = self.tell()?;

        // Pointers in AVI are 32 bit. Do not write beyond that else the whole AVI file will be corrupted (not playable).
        // Index entry size: 16 bytes (for each frame)
        if frame_pos + jpeg_data.len() as u64 + (self.frames*16) as u64 > 4200000000 { // 2^32 = 4 294 967 296
            return Err(err("File is too large"));
        }

        self.frames += 1;

        self.write_u32(0x63643030)?;    // "00dc" compressed frame
        self.write_length_field()?;     // Chunk length (nesting level 2)
        self.avif.write(jpeg_data)?;
        self.finalize_length_field()?;  // "00dc" chunk finished (nesting level 2)

        // Write index data
        self.idx_u32(0x63643030);                   // "00dc" compressed frame
        self.idx_u32(0x10);                         // flags: select AVIIF_KEYFRAME (The flag indicates key frames in the video sequence. Key frames do not need previous video information to be decompressed.)
        self.idx_u32((frame_pos - self.movi_pos) as u32); // offset to the chunk, offset can be relative to file start or 'movi'
        self.idx_u32(jpeg_data.len() as u32);         // length of the chunk

        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        self.finalize_length_field()?; // LIST 'movi' finished (nesting level 1)

        // Write index
        self.write_str("idx1")?; // idx1 chunk
        let idx_len = self.idx.len();
        self.write_u32(idx_len as u32)?; // Chunk length (we know its size, no need to use writeLengthField() and finalizeLengthField() pair)
        // Copy temporary index data
        self.avif.write(&self.idx)?;

        let pos = self.tell()?;
        self.avif.seek(std::io::SeekFrom::Start(self.frames_count_field_pos))?;
        self.write_u32(self.frames)?;
        self.avif.seek(std::io::SeekFrom::Start(self.frames_count_field_pos2))?;
        self.write_u32(self.frames)?;
        self.avif.seek(std::io::SeekFrom::Start(pos))?;

        self.finalize_length_field()?; // 'RIFF' File finished (nesting level 0)

        Ok(())
    }

    pub fn destroy(&mut self) {
        let r = self.finalize();

        match r {
            Ok(_) => (),
            Err(e) => {
                println!("Warning: mjpeg writer error: {}", e);
            }
        }
    }
}

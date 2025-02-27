# nfo2tags

I wanted to get the data from my tinymediamanager generated NFO files into the files. I also wanted the poster to be put in the file. **This will clear your existing tags from the file**

## Arguments

**-v** or **--video** Sets the video file or the folder where the video files are found.
**-n** or ** --nfo** Sets the .nfo file. This only applys to single file use. In folder mode it looks for .nfo files with the same name as the movie.
**-c** or **--cover** Sets the cover file, either jpg or png. If using folder mode, this does not work. It will use the video file name + texted passed in to -N or --cover-name. Default is '-poster'.
**-N** or **--cover-name** This is a custom suffix for the cover file. It will be added to the video file name to identify the image you want to use.
**-o** or **--output** Sets mp4's output file, since the whole container must be rewritten to put in the tags. If missing, it just creats a backup of the file, File.OLD.mp4. ***Does not apply to MVK***
 
## What to Expect
It acts different for each file type. MKV files can be edited directly. However, MP4 container must be rewritten to put in the tags. Also, MP4 are added from memory, while MKV files are added from a created XML file.

When using with a folder, it handles each file as it comes accross it. Now you must be careful, since mp4 must duplicate, you must make sure there is space to do this.

## Free

I am starting my IT company backup. I am trying to raise some funds. If you like the software, please contribute.
Zelle bmoore@tekgnosis.works
venmo @tekgnosis

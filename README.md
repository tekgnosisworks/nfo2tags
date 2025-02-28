# nfo2tags

I wanted to get some of the data from my tinymediamanager generated NFO files into the files. I also wanted the poster to be put in the file for thumbnailing. I also wanted all the old "tags" to be removed so **NOTE: This will clear your existing tags from the file**

**IMPORTANT** You must have ffmpeg(https://www.ffmpeg.org/) and mkvpropedit(https://mkvtoolnix.download/) installed and part of path for this to work.

## Tags
These are limited by the standards and implementations of the containers.

**MP4:** Title, Genre, Keywords, Description, Synopsis, Premiered(date)

**MKV:** Actors, Directors, Credits, Description, Summary, Collection Name, COllection Overview, Plot, Outline, Overview, Tags, Genre, id (imdb), Title, OriginalTitle, Year, Tagline, Runtime, MPAA, Certification, tmdbid, Country, Premiered (date), Studio

## Arguments

**-v** or **--video** Sets the video file or the folder where the video files are found.\
**-n** or **--nfo** Sets the .nfo file. This only applys to single file use. In folder mode it looks for .nfo files with the same name as the movie.\
**-c** or **--cover** Sets the cover file, either jpg or png. If using folder mode, this does not work. It will use the video file name + texted passed in to -N or --cover-name. Default is '-poster'.\
**-N** or **--cover-name** This is a custom suffix for the cover file. It will be added to the video file name to identify the image you want to use.\
**-o** or **--output** Sets mp4's output file, since the whole container must be rewritten to put in the tags. If missing, it just creats a backup of the file, File.OLD.mp4. ***Does not apply to MVK***

**Use after testing your stuff**
I did over thousands of videos with this working great. But it does not go in your trash when deleted this way. It is permenant.\
**-d** or **--delete** This tells it to delete the original MP4 file after it created the tagged file.\

 
## What to Expect
It acts different for each file type. MKV files can be edited directly, so are fast. However, MP4 container must be recreated to put in the tags. So it streams the orignial streams into a new container. Also, MP4 are added from memory, while MKV files are added from a created XML file.

When using with a folder mode, it handles each file as it comes accross it. Be storage aware, since mp4 must duplicate the file. You must make sure there is space to do this.

**Logging** It posts the log in the terminal and to nfo2tags.log file adjacent to executable.

## Free

I am starting my IT company backup. I am trying to raise some funds. If you like the software, please contribute.\
Zelle bmoore@tekgnosis.works\
venmo @tekgnosis

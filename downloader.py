import requests

import asyncio
import aiohttp
import os
import re

# beatmap_base_url = "https://beatconnect.io/b/{}"
beatmap_base_url = "https://api.nerinyan.moe/d/{}"
# beatmap_base_url = "https://osu.direct/api/d/{}"
# beatmap_base_url = "https://catboy.best/d/{}"
# beatmap_base_url = "https://osu.ppy.sh/beatmapsets/{}/download"

# Maximum number of retry attempts
MAX_RETRIES = 3
# Delay between retries in seconds
RETRY_DELAY = 30


async def get_filename_from_response(response):
    # Get the Content-Disposition header
    content_disposition = response.headers.get('Content-Disposition')

    if content_disposition:
        # Look for the filename in the header
        match = re.search(r'filename="([^"]+)"', content_disposition)
        if match:
            return match.group(1)  # Return the filename
    return None  # If no filename is found


# Function to download a single file
async def download_file(session, beatmap, dest_folder, extension):
    # Retry logic
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            url = beatmap_base_url.format(beatmap)

            # Fetch the file
            async with session.get(url) as response:
                # Check if the request was successful
                if response.status == 200:
                    # Get the file name from the URL
                    file_name = await get_filename_from_response(response)
                    if not file_name:
                        file_name = f"{beatmap}.{extension}"
                    file_name = os.path.join(dest_folder, file_name)
                    # Write the content to a file
                    with open(f"{file_name}", 'wb') as f:
                        f.write(await response.read())
                    print(f"Downloaded: {file_name}")
                    return
                else:
                    print(
                        f"Failed to download {url}, status code: {response.status}")
        except Exception as e:
            print(f"Error downloading {url}: {str(e)}")

        # If there was an error, wait before retrying
        if attempt < MAX_RETRIES:
            print(f"Retrying {url} in {RETRY_DELAY} seconds...")
            await asyncio.sleep(RETRY_DELAY)


# Function to handle concurrent downloads
async def download_concurrently(beatmaps, dest_folder, max_concurrent_downloads=5):
    # Create an aiohttp session
    async with aiohttp.ClientSession() as session:
        # Create a semaphore to limit the number of concurrent downloads
        semaphore = asyncio.Semaphore(max_concurrent_downloads)

        # Wrap download_file with semaphore to limit concurrency
        async def sem_download(beatmap):
            async with semaphore:
                await download_file(session, beatmap, dest_folder, 'osz')

        # Create a list of tasks for each download
        tasks = [sem_download(beatmap) for beatmap in beatmaps]

        # Wait for all downloads to finish
        await asyncio.gather(*tasks)


# Destination folder where the files will be saved
destination_folder = 'D:\\osu!\\Songs'

# Ensure the destination folder exists
os.makedirs(destination_folder, exist_ok=True)


beatmaps = open('output').read().split()


# Run the concurrent download process
asyncio.run(download_concurrently(beatmaps, destination_folder))

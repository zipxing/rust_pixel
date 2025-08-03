import os
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed

# 假设你的 URL 列表
image_urls = [
"https://pbs.twimg.com/media/GfpR-K6W0AAPZFD.png",
"https://pbs.twimg.com/media/GfeqtChXcAAGt8L.png",
"https://pbs.twimg.com/media/GfVOm3tWAAAWP13.png",
"https://pbs.twimg.com/media/Ge687_PW0AAbbAG.png",
"https://pbs.twimg.com/media/Gewm00zWcAAL2KI.png",
"https://pbs.twimg.com/media/GegoSyDXcAEAMXH.png",
"https://pbs.twimg.com/media/GeSpu5eWQAADSGw.png",
"https://pbs.twimg.com/media/GeInK9UWYAAqK6v.png",
"https://pbs.twimg.com/media/GeHjgFXX0AAM66e.png",
"https://pbs.twimg.com/media/Gd4QR6oW4AAwu4i.png",
"https://pbs.twimg.com/media/GdooEOcWcAAg4S8.png",
"https://pbs.twimg.com/media/GdUe1SfWAAAzeum.png",
"https://pbs.twimg.com/media/GdUeW3kWAAAl3nQ.png",
"https://pbs.twimg.com/media/GdUd4qMXgAAeywS.png",
"https://pbs.twimg.com/media/GdUdhmnXMAAsEtO.png",
"https://pbs.twimg.com/media/GdUdIGGWMAAjlb1.png",
"https://pbs.twimg.com/media/GdJgeZPWEAAwIp4.png",
"https://pbs.twimg.com/media/GdJf-gCXUAETckd.png",
"https://pbs.twimg.com/media/GdJewzZXYAAHTrt.png",
"https://pbs.twimg.com/media/GdJOGAYXEAAXJPV.png",
"https://pbs.twimg.com/media/GdJNlf7XoAAcsi0.png",
"https://pbs.twimg.com/media/GdJNLS3XIAEK_XJ.png",
"https://pbs.twimg.com/media/GdJMr3xXsAAiZBq.png",
"https://pbs.twimg.com/media/GdJMD0UW8AAwd4M.png",
"https://pbs.twimg.com/media/GdGIEcTWcAArDeB.png",
"https://pbs.twimg.com/media/GdF6XrpWIAAj2gq.png",
"https://pbs.twimg.com/media/GdF36GxWEAAnID7.png",
"https://pbs.twimg.com/media/GdFnfRgXQAAyzgr.png",
"https://pbs.twimg.com/media/GdFaQawW4AA6eiC.png",
"https://pbs.twimg.com/media/GdAXm7hXQAAa6fs.png",
"https://pbs.twimg.com/media/GcXB7AaWoAAr4N1.png",
"https://pbs.twimg.com/media/GcMupAxXUAA9MwQ.png",
"https://pbs.twimg.com/media/GcH-YOtWgAAp9KO.png",
"https://pbs.twimg.com/media/Gb310CbWYAAVeru.png",
"https://pbs.twimg.com/media/GbywT6HWgAc8JzH.png",
"https://pbs.twimg.com/media/Gbt3AuFX0BEafay.png",
"https://pbs.twimg.com/media/Gbt2f9KXMAIbVTY.png",
"https://pbs.twimg.com/media/Gbt2BntWsAAhUbp.png",
"https://pbs.twimg.com/media/Gboe0Z_W4AAEy6K.png",
"https://pbs.twimg.com/media/GboeJzcXAAAOnWi.png",
"https://pbs.twimg.com/media/GbodlZnWMAArZMd.png",
"https://pbs.twimg.com/media/Gboc0oKXwAAsTum.png",
"https://pbs.twimg.com/media/GbocWOzXMAEpHF5.png",
"https://pbs.twimg.com/media/Gbob3krXwAAnVli.png",
"https://pbs.twimg.com/media/GbdWQWjbEAAMb6f.png",
"https://pbs.twimg.com/media/GbdVt9RbEAAhkGX.png",
"https://pbs.twimg.com/media/GbbnWXjbwAEAtpI.png",
"https://pbs.twimg.com/media/Gbbmp2ea8AA9c9M.png",
"https://pbs.twimg.com/media/GbaH0BLasAAjPoK.png",
"https://pbs.twimg.com/media/GbaHBosaUAAZcRg.png",
"https://pbs.twimg.com/media/GbZ0C5Pa8AE9Otk.png",
"https://pbs.twimg.com/media/GbZn4bpbkAA4XeF.png",
"https://pbs.twimg.com/media/GbZlqqja8AAL4kR.png",
"https://pbs.twimg.com/media/GbYu4I9WcAAXI47.png",
"https://pbs.twimg.com/media/Ga_ru3dWkAANzmN.png",
"https://pbs.twimg.com/media/Ga1gJQ1WUAAqW9d.png",
"https://pbs.twimg.com/media/GaqiohmXYAAOpyH.png",
"https://pbs.twimg.com/media/GaajBKYX0AA8Gem.png",
"https://pbs.twimg.com/media/GaW8-XlWQAAfoew.png",
"https://pbs.twimg.com/media/GaW8LQBWoAA1ip4.png",
"https://pbs.twimg.com/media/GaW7hEvX0AAjZRs.png",
"https://pbs.twimg.com/media/GaWyMw2WwAA3x5w.png",
"https://pbs.twimg.com/media/GaVkZI7WkAA9WPg.png",
"https://pbs.twimg.com/media/GaVg1M1W4AE89j-.png",
"https://pbs.twimg.com/media/GaLrqFMWYAAA7NS.png",
"https://pbs.twimg.com/media/GaLrIsTXoAA9_l-.png",
"https://pbs.twimg.com/media/GaF08etXgAA5taD.png",
"https://pbs.twimg.com/media/GaBRNnZW4AAtRkR.png",
"https://pbs.twimg.com/media/GZ_8qPGWMAAcXgS.png",
"https://pbs.twimg.com/media/GZ5nboVWAAAlmJd.png",
"https://pbs.twimg.com/media/GZ5lxRQXMAAwSUb.png",
"https://pbs.twimg.com/media/GZxHKmnXsAE701R.png",
"https://pbs.twimg.com/media/GZiJ44WXMAIQWii.png",
"https://pbs.twimg.com/media/GZYCcn4XcAAZZIG.png",
"https://pbs.twimg.com/media/GZXn-_OWwAA9_2V.png",
"https://pbs.twimg.com/media/GZTYgmcWgAE6Qla.png",
"https://pbs.twimg.com/media/GZNfitcW4AAVe1_.png",
"https://pbs.twimg.com/media/GZNfLITWkAAav0V.png",
"https://pbs.twimg.com/media/GZNeuoXWYAAYEWz.png",
"https://pbs.twimg.com/media/GZMQs9pWgAAD3c0.png",
"https://pbs.twimg.com/media/GZMQPOUXAAAEl5l.png",
"https://pbs.twimg.com/media/GZMPyfVW0AAgaQ8.png",
"https://pbs.twimg.com/media/GZMPSQpWMAA7gyN.png",
"https://pbs.twimg.com/media/GZMO23TW0AAkxUq.png",
"https://pbs.twimg.com/media/GZJwyx_WYAAzhrh.png",
"https://pbs.twimg.com/media/GZJvy2cWYAArtfy.png",
"https://pbs.twimg.com/media/GZJu_sJW4AA4q-w.png",
"https://pbs.twimg.com/media/GZJunTLXAAAp6SO.png",
"https://pbs.twimg.com/media/GZJuJ96XkAAOQ2-.png",
"https://pbs.twimg.com/media/GY0y_ZCWEAAJOhc.png",
"https://pbs.twimg.com/media/GYy4MtQXIAAlV9O.png",
"https://pbs.twimg.com/media/GYy3nqjWoAAO5H9.png",
"https://pbs.twimg.com/media/GYv5RXGXUAAUWqK.png",
"https://pbs.twimg.com/media/GYeZ4RRWwAAPP9Z.png",
"https://pbs.twimg.com/media/GYaLYiBaMAMNH-Q.png",
"https://pbs.twimg.com/media/GX7TRT7XYAAvRXN.png",
"https://pbs.twimg.com/media/GXqr4C2WoAAOg2k.png",
"https://pbs.twimg.com/media/GXl9f69WkAAqJaj.png",
"https://pbs.twimg.com/media/GXg7FBiXYAA0Bfq.png",
    # ... 更多 URL
]

# 保存目录
save_dir = "downloaded_images"
os.makedirs(save_dir, exist_ok=True)

# 下载函数
def download_image(url, save_path):
    try:
        response = requests.get(url, timeout=10)
        if response.status_code == 200:
            with open(save_path, 'wb') as f:
                f.write(response.content)
            print(f"✅ Saved: {save_path}")
            return True
        else:
            print(f"❌ Failed: {url} (Status: {response.status_code})")
            return False
    except Exception as e:
        print(f"❌ Error downloading {url}: {e}")
        return False

# 下载任务包装函数
def download_task(args):
    idx, url = args
    ext = os.path.splitext(url)[1].split("?")[0] or ".png"  # 获取扩展名
    filename = f"img_{idx:04d}{ext}"
    save_path = os.path.join(save_dir, filename)
    return download_image(url, save_path)

# 多线程批量下载
def download_images_concurrently(image_urls, max_workers=5):
    """
    使用多线程并发下载图片
    
    Args:
        image_urls: 图片URL列表
        max_workers: 最大线程数，默认为5
    """
    print(f"开始下载 {len(image_urls)} 张图片，使用 {max_workers} 个线程...")
    
    # 准备下载任务
    download_tasks = [(idx, url) for idx, url in enumerate(image_urls, start=1)]
    
    successful_downloads = 0
    failed_downloads = 0
    
    # 使用线程池执行下载任务
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        # 提交所有任务
        future_to_task = {executor.submit(download_task, task): task for task in download_tasks}
        
        # 获取结果
        for future in as_completed(future_to_task):
            task = future_to_task[future]
            try:
                result = future.result()
                if result:
                    successful_downloads += 1
                else:
                    failed_downloads += 1
            except Exception as exc:
                idx, url = task
                print(f"❌ Task {idx} generated an exception: {exc}")
                failed_downloads += 1
    
    print(f"\n下载完成！")
    print(f"✅ 成功: {successful_downloads} 张")
    print(f"❌ 失败: {failed_downloads} 张")

# 开始下载
download_images_concurrently(image_urls, max_workers=8)

import subprocess
import time
import multiprocessing
from concurrent.futures import ProcessPoolExecutor, as_completed
import os

# 确保输出目录存在
os.makedirs("assets/pix", exist_ok=True)

def convert_image(img_num):
    """
    转换单张图片的函数
    
    Args:
        img_num: 图片编号
    
    Returns:
        tuple: (img_num, success, error_msg)
    """
    try:
        input_file = f"downloaded_images/img_{img_num:04d}.png"
        output_file = f"assets/pix/img_{img_num:04d}.pix"
        
        # 检查输入文件是否存在
        if not os.path.exists(input_file):
            return (img_num, False, f"Input file {input_file} not found")
        
        command = f"./petii {input_file} 40 25 true 32 41 320 200 > {output_file}"
        
        result = subprocess.run(command, shell=True, capture_output=True, text=True)
        
        if result.returncode == 0:
            print(f"✅ img_{img_num:04d} completed")
            return (img_num, True, None)
        else:
            error_msg = result.stderr.strip() if result.stderr else f"Command failed with return code {result.returncode}"
            print(f"❌ img_{img_num:04d} failed: {error_msg}")
            return (img_num, False, error_msg)
            
    except Exception as e:
        error_msg = str(e)
        print(f"❌ img_{img_num:04d} exception: {error_msg}")
        return (img_num, False, error_msg)

def convert_images_parallel(start_num=1, end_num=100, max_workers=None):
    """
    并行转换图片
    
    Args:
        start_num: 开始图片编号
        end_num: 结束图片编号（不包含）
        max_workers: 最大进程数，默认为CPU核心数
    """
    if max_workers is None:
        max_workers = multiprocessing.cpu_count()
    
    print(f"开始并行转换图片 {start_num:04d} 到 {end_num-1:04d}")
    print(f"使用 {max_workers} 个进程，CPU核心数: {multiprocessing.cpu_count()}")
    
    img_numbers = list(range(start_num, end_num))
    successful_conversions = 0
    failed_conversions = 0
    
    start_time = time.time()
    
    # 使用进程池并行处理
    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        # 提交所有任务
        future_to_img = {executor.submit(convert_image, img_num): img_num for img_num in img_numbers}
        
        # 获取结果
        for future in as_completed(future_to_img):
            img_num = future_to_img[future]
            try:
                img_num_result, success, error_msg = future.result()
                if success:
                    successful_conversions += 1
                else:
                    failed_conversions += 1
            except Exception as exc:
                print(f"❌ img_{img_num:04d} generated an exception: {exc}")
                failed_conversions += 1
    
    end_time = time.time()
    total_time = end_time - start_time
    
    print(f"\n转换完成！")
    print(f"✅ 成功: {successful_conversions} 张")
    print(f"❌ 失败: {failed_conversions} 张")
    print(f"⏱️ 总耗时: {total_time:.2f} 秒")
    print(f"📈 平均速度: {(successful_conversions + failed_conversions) / total_time:.2f} 张/秒")

# 开始转换
if __name__ == "__main__":
    repeat_times = 100
    convert_images_parallel(1, repeat_times)

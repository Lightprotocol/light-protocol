#!/usr/bin/env python3

import asyncio
import subprocess
import json
import os
import sys
from dotenv import load_dotenv
from manage_droplets import create_droplet, delete_droplet, update_inventory, manager

load_dotenv()

MIN_INSTANCES = 1
MAX_INSTANCES = 10
SCALE_UP_THRESHOLD = 20  # 20% queue fullness
SCALE_UP_RATE_THRESHOLD = 5  # 5% increase per minute
CHECK_INTERVAL = 60  # seconds
HISTORY_LENGTH = 3  # number of checks to keep in history

# Use the local Forester build
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(os.path.dirname(SCRIPT_DIR))  # Go up two levels
FORESTER_PATH = os.path.join(ROOT_DIR, "target", "release", "forester")

print(f"Looking for Forester at: {FORESTER_PATH}")

if not os.path.exists(FORESTER_PATH):
    print(f"Forester binary not found. Attempting to build...")
    try:
        subprocess.run(["cargo", "build", "--release"], cwd=ROOT_DIR, check=True)
        print("Forester built successfully.")
    except subprocess.CalledProcessError:
        print("Failed to build Forester. Please build it manually.")
        sys.exit(1)

if not os.path.exists(FORESTER_PATH):
    print(f"Error: Forester binary still not found at {FORESTER_PATH}")
    sys.exit(1)

async def get_queue_status():
    try:
        result = subprocess.run([FORESTER_PATH, 'status'], capture_output=True, text=True, check=True)
        # Parse the output manually
        lines = result.stdout.strip().split('\n')
        status = {}
        current_queue = None
        for line in lines:
            if line.startswith('State Queue:') or line.startswith('Address Queue:'):
                current_queue = line.split(':')[0].strip().lower().replace(' ', '_')
                status[current_queue] = {}
            elif ':' in line and current_queue:
                key, value = line.split(':', 1)
                key = key.strip().lower().replace(' ', '_')
                value = value.strip()
                if key == 'fullness':
                    value = float(value.rstrip('%'))
                status[current_queue][key] = value
        return status
    except subprocess.CalledProcessError as e:
        print(f"Error running Forester: {e}")
        print(f"Forester stderr: {e.stderr}")
        sys.exit(1)
    except Exception as e:
        print(f"Error parsing Forester output: {e}")
        print(f"Forester stdout: {result.stdout}")
        sys.exit(1)

def calculate_scaling_decision(queue_history):
    current_fullness = queue_history[-1]
    if len(queue_history) > 1:
        rate_of_change = (current_fullness - queue_history[-2]) / (CHECK_INTERVAL / 60)  # % per minute
    else:
        rate_of_change = 0

    if current_fullness > SCALE_UP_THRESHOLD or rate_of_change > SCALE_UP_RATE_THRESHOLD:
        return 1  # Scale up
    elif current_fullness < 10 and rate_of_change < 0:
        return -1  # Scale down
    else:
        return 0  # No change

async def scale_instances(current_count, target_count):
    if current_count < target_count:
        for i in range(target_count - current_count):
            droplet = create_droplet(f'forester-node-{current_count + i}')
            print(f'Created droplet: {droplet.name} (IP: {droplet.ip_address})')
    elif current_count > target_count:
        droplets = manager.get_all_droplets()
        for droplet in droplets[target_count:current_count]:
            delete_droplet(droplet.id)
            print(f'Deleted droplet: {droplet.name}')
    
    update_inventory(manager.get_all_droplets())
    subprocess.run(['npm', 'run', 'deploy'])

async def auto_scale():
    queue_history = {'state_queue': [], 'address_queue': []}  # Initialize history for each queue
    while True:
        status = await get_queue_status()
        current_count = len(manager.get_all_droplets())

        scaling_decision = 0
        for queue, data in status.items():
            if 'fullness' in data:
                fullness = data['fullness']
                queue_history[queue].append(fullness)
                if len(queue_history[queue]) > HISTORY_LENGTH:
                    queue_history[queue].pop(0)
                
                queue_scaling = calculate_scaling_decision(queue_history[queue])
                scaling_decision = max(scaling_decision, queue_scaling)

        if scaling_decision > 0 and current_count < MAX_INSTANCES:
            target_count = min(current_count + 1, MAX_INSTANCES)
            await scale_instances(current_count, target_count)
        elif scaling_decision < 0 and current_count > MIN_INSTANCES:
            target_count = max(current_count - 1, MIN_INSTANCES)
            await scale_instances(current_count, target_count)

        await asyncio.sleep(CHECK_INTERVAL)

if __name__ == '__main__':
    asyncio.run(auto_scale())
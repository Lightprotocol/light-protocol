#!/usr/bin/env python3

import os
import sys
import digitalocean
from dotenv import load_dotenv

load_dotenv()

DO_TOKEN = os.getenv('DIGITALOCEAN_TOKEN')
manager = digitalocean.Manager(token=DO_TOKEN)

def create_droplet(name):
    droplet = digitalocean.Droplet(
        token=DO_TOKEN,
        name=name,
        region='nyc3',
        image='ubuntu-20-04-x64',
        size_slug='s-1vcpu-1gb',
        ssh_keys=manager.get_all_sshkeys(),
    )
    droplet.create()
    droplet.load()
    return droplet

def delete_droplet(droplet_id):
    droplet = digitalocean.Droplet(token=DO_TOKEN, id=droplet_id)
    droplet.destroy()

def update_inventory(droplets):
    with open('forester/ansible/inventory.ini', 'w') as f:
        f.write('[forester_nodes]\n')
        for droplet in droplets:
            f.write(f'{droplet.ip_address}\n')

def main(action, count=1):
    if action == 'create':
        for i in range(int(count)):
            droplet = create_droplet(f'forester-node-{i}')
            print(f'Created droplet: {droplet.name} (IP: {droplet.ip_address})')
    elif action == 'delete':
        droplets = manager.get_all_droplets()
        for droplet in droplets[:int(count)]:
            delete_droplet(droplet.id)
            print(f'Deleted droplet: {droplet.name}')
    elif action == 'update':
        droplets = manager.get_all_droplets()
        update_inventory(droplets)
        print('Updated inventory file')

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print('Usage: python manage_droplets.py [create|delete|update] [count]')
        sys.exit(1)
    action = sys.argv[1]
    count = sys.argv[2] if len(sys.argv) > 2 else 1
    main(action, count)